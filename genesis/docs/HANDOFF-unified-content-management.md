# Handoff: Unified Content Management System

**Date**: 2025-01-25
**Context**: Path thumbnail investigation revealed that static assets are not stored in elohim-storage, exposing a broader architectural gap.

---

## Vision: Digital Sovereignty Through Unified Storage

The goal is a **"super Google Drive"** - a unified interface where humans can:

1. **Own their entire digital footprint** across all life contexts (files, photos, videos, books, learning, economics, identity, community)
2. **See at a glance** the status of every piece of content:
   - **Safety**: Is it encrypted? Private? Public?
   - **Reach**: Who can see it? How far has it spread?
   - **Replication**: How safe is it? Where are the copies?
   - **Context**: Which app/life-area does it belong to?
3. **Trust the system** to protect, distribute, and preserve their content according to their preferences
4. **Break free from silos** - no more fragmentation across Google/Apple/Amazon/Netflix/Spotify ecosystems

This is not just "file storage" - it's **personal data infrastructure** that:
- Survives platform shutdowns
- Can be inherited by future generations
- Respects privacy by default
- Enables selective sharing with granular control
- Leverages community (support network, households) for resilience

---

## Problem Statement (Current State)

Content in the Elohim ecosystem is fragmented across multiple storage mechanisms:

| Content Type | Current Storage | Issues |
|--------------|-----------------|--------|
| Path thumbnails | Static files in `public/images/` | Requires app rebuild/redeploy |
| Content body text | elohim-storage blobs | Works correctly |
| Path JSON definitions | Seeded from `genesis/data/` | No runtime editing |
| HTML5 apps | elohim-storage blobs (ZIPs) | Works correctly |
| User-uploaded content | **Not implemented** | No upload capability |

The vision is a **unified content management experience** like Google Drive or S3, where users can:
- Upload, organize, and manage all their files
- Understand file status at a glance (safety, reach, replication)
- Trust that their content is appropriately secured and distributed

---

## Current CRUD State Assessment

### elohim-storage Backend

| Entity | Create | Read | Update | Delete | Notes |
|--------|--------|------|--------|--------|-------|
| Content | Bulk | Yes | No | No | Seeder-focused |
| Paths | Bulk | Yes | No | No | Seeder-focused |
| Relationships | Yes | Yes | No | No | |
| Blobs | Yes | Yes | No | No | Content-addressed, immutable |
| Presences | Yes | Yes | Partial | No | Stewardship/claim actions |
| Events | Yes | Yes | No | No | Append-only by design |
| Mastery | Upsert | Yes | Via upsert | No | |
| Allocations | Yes | Yes | Yes | Yes | Full CRUD |

### Angular Frontend Services

- **StorageClientService** (`storage-client.service.ts`):
  - Blob fetch, content query, path query
  - Bulk create for seeding
  - `getPathThumbnailUrl()` already supports `thumbnailBlobHash` fallback!

- **StorageApiService** (`storage-api.service.ts`):
  - Rich query APIs for relationships, presences, events, mastery
  - Create operations for most entities
  - Limited update/delete (only allocations)

### What's Missing for User-Managed Content

1. **Single-item Create endpoints** (not just bulk)
2. **Update endpoints** for content, paths
3. **Delete endpoints** for content, paths
4. **Blob upload** from browser (currently only seeder uploads)
5. **File manager UI** component
6. **Metadata editing** UI for content properties

---

## Content Dimensions Model

Every piece of content should expose **four** key dimension categories:

### 4. Application Context Dimension

What app/context does this content belong to?

```
┌─────────────────────────────────────────────────────────────────┐
│                 APPLICATION CONTEXT MODEL                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  elohim-storage is NOT just for lamad learning content.        │
│  It's the unified storage layer for a person's entire          │
│  digital life - replacing fragmented cloud services:           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                                                             ││
│  │  TODAY'S FRAGMENTATION          ELOHIM UNIFIED STORAGE      ││
│  │  ════════════════════          ══════════════════════       ││
│  │                                                             ││
│  │  Google Drive ─────────┐                                    ││
│  │  Dropbox ──────────────┼──►  📁 PERSONAL FILES              ││
│  │  OneDrive ─────────────┘       (documents, spreadsheets)    ││
│  │                                                             ││
│  │  Google Photos ────────┐                                    ││
│  │  iCloud Photos ────────┼──►  📷 PHOTOS & ALBUMS             ││
│  │  Photoprism ───────────┘       (memories, family, events)   ││
│  │                                                             ││
│  │  Netflix ──────────────┐                                    ││
│  │  Plex ─────────────────┼──►  🎬 VIDEO LIBRARY               ││
│  │  Jellyfin ─────────────┘       (movies, shows, home video)  ││
│  │                                                             ││
│  │  Spotify ──────────────┐                                    ││
│  │  Apple Podcasts ───────┼──►  🎧 AUDIO LIBRARY               ││
│  │  Pocket Casts ─────────┘       (podcasts, music, ambient)   ││
│  │                                                             ││
│  │  Audible ──────────────┐                                    ││
│  │  Libby ────────────────┼──►  📚 BOOK LIBRARY                ││
│  │  Kindle ───────────────┤       (ebooks, audiobooks)         ││
│  │  Google Books ─────────┘                                    ││
│  │                                                             ││
│  │  Coursera ─────────────┐                                    ││
│  │  Udemy ────────────────┼──►  🎓 LAMAD (Learning)            ││
│  │  Khan Academy ─────────┘       (courses, paths, mastery)    ││
│  │                                                             ││
│  │  (Future contexts...)                                       ││
│  │  ──────────────────────────►  💰 SHEFA (Economic)           ││
│  │  ──────────────────────────►  👤 IMAGODEI (Identity)        ││
│  │  ──────────────────────────►  🏛️ QAHAL (Community)          ││
│  │                                                             ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  DESIGN PRINCIPLES:                                             │
│                                                                 │
│  1. UNIFIED SUBSTRATE                                           │
│     Same blob storage, same metadata schema, same dimensions    │
│     (safety, reach, replication) - regardless of app context    │
│                                                                 │
│  2. CONTEXT-AWARE UI                                            │
│     Each app presents content appropriately:                    │
│     - Photos app: gallery grid, timeline, faces, albums         │
│     - Video app: poster art, watch progress, recommendations    │
│     - Books app: cover art, reading progress, annotations       │
│     - Lamad: learning paths, mastery tracking, quizzes          │
│                                                                 │
│  3. CROSS-CONTEXT RELATIONSHIPS                                 │
│     Content can span contexts:                                  │
│     - A photo from a family event → referenced in a learning    │
│       path about family history                                 │
│     - An audiobook → source for a book club community           │
│     - A podcast episode → cited in research notes               │
│                                                                 │
│  4. OWNERSHIP CONTINUITY                                        │
│     Unlike siloed services:                                     │
│     - No vendor lock-in (your data, your nodes)                 │
│     - Export/migrate entire contexts                            │
│     - Survive service shutdowns                                 │
│     - Inherit to next generation                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**App Context Registry**:

| Context ID | Display Name | Icon | Content Types | Special Features |
|------------|--------------|------|---------------|------------------|
| `files` | Personal Files | 📁 | `document`, `spreadsheet`, `presentation`, `archive` | Folder hierarchy, version history |
| `photos` | Photos & Albums | 📷 | `photo`, `album`, `video-clip` | Face detection, location, timeline |
| `video` | Video Library | 🎬 | `movie`, `episode`, `home-video` | Watch progress, subtitles, transcoding |
| `audio` | Audio Library | 🎧 | `podcast`, `music`, `audiobook`, `ambient` | Playback position, playlists |
| `books` | Book Library | 📚 | `ebook`, `audiobook`, `pdf-book` | Reading position, annotations, highlights |
| `lamad` | Learning | 🎓 | `concept`, `article`, `quiz`, `path` | Mastery tracking, spaced repetition |
| `shefa` | Economic | 💰 | `transaction`, `agreement`, `flow` | ValueFlows integration |
| `imagodei` | Identity | 👤 | `profile`, `attestation`, `presence` | Verification, claims |
| `qahal` | Community | 🏛️ | `post`, `discussion`, `event` | Governance, consensus |

**UI Considerations**:
- Global search across all contexts
- Context switcher in navigation (like Google apps grid)
- "Add to..." action to link content across contexts
- Unified storage quota display across all contexts
- Context-specific viewers/editors with shared dimension controls

---

The remaining three dimensions apply universally across all app contexts:

### 1. Safety Dimensions

How protected is this content?

```
┌─────────────────────────────────────────────────────────────────┐
│                      SAFETY SPECTRUM                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ENCRYPTED ◄─────────────────────────────────────────► PUBLIC   │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐│
│  │ Password    │  │ Private     │  │ Unlisted    │  │ Public  ││
│  │ Protected   │  │ (owner/ACL) │  │ (link only) │  │ Discover││
│  │ + Encrypted │  │             │  │             │  │ -able   ││
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘│
│                                                                 │
│  Key escrow options for encrypted content:                      │
│  - Personal only (no recovery)                                  │
│  - Support network key shares (threshold recovery)              │
│  - Doorway custodial recovery                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**UI Indicators**:
- Lock icon variants (padlock, shield, globe)
- Color coding (red → orange → yellow → green)
- Tooltip with encryption method and recovery options

### 2. Reach Dimensions

Who can see/access this content? (Social-reach integration)

```
┌─────────────────────────────────────────────────────────────────┐
│                       REACH MODEL                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Social reach determines content visibility in the network:     │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    CONCENTRIC CIRCLES                       ││
│  │                                                             ││
│  │                    ┌───────────────┐                        ││
│  │                    │   COMMONS     │  Global discovery      ││
│  │                 ┌──┴───────────────┴──┐                     ││
│  │                 │    COMMUNITY        │  Your communities   ││
│  │              ┌──┴─────────────────────┴──┐                  ││
│  │              │      SUPPORT NETWORK      │  Trusted circle  ││
│  │           ┌──┴───────────────────────────┴──┐               ││
│  │           │         HOUSEHOLD               │  Family/close ││
│  │        ┌──┴─────────────────────────────────┴──┐            ││
│  │        │              PERSONAL                  │  Only you ││
│  │        └────────────────────────────────────────┘           ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  Content "reach" expands based on:                              │
│  - Explicit sharing actions                                     │
│  - Community membership propagation                             │
│  - Recognition/attestation chains                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**UI Indicators**:
- Concentric circle visualization
- "Visible to X people/communities" count
- Reach expansion history ("Shared to Community X on date")

### 3. Replication Dimensions

How durable/available is this content?

```
┌─────────────────────────────────────────────────────────────────┐
│                    REPLICATION TIERS                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  RISK ◄──────────────────────────────────────────► DURABILITY  │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ TIER 0: Device Only                                         ││
│  │ ⚠️ HIGH RISK - No backup, device loss = data loss           ││
│  │ Indicator: Red warning, single device icon                  ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ TIER 1: Device + Personal Node                              ││
│  │ Personal backup - synced to your always-on node             ││
│  │ Indicator: Yellow, two-device icon                          ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ TIER 2: Support Network Replicated                          ││
│  │ Trusted circle holds encrypted shards                       ││
│  │ Indicator: Blue, network icon with shard count              ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ TIER 3: Doorway Account Recovery                            ││
│  │ Full recovery possible through doorway infrastructure       ││
│  │ Indicator: Green checkmark, doorway icon                    ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ═══════════════════════════════════════════════════════════════│
│                                                                 │
│  HIGH AVAILABILITY OVERLAY (on top of any tier):                │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ P2P Replication Stats:                                      ││
│  │ • Peer replica count: 12 nodes                              ││
│  │ • Geographic distribution: 4 regions                        ││
│  │ • Reed-Solomon shards: 8 data + 4 parity                    ││
│  │                                                             ││
│  │ CDN Edge Status (social-reach powered):                     ││
│  │ • Edge nodes: 23 locations                                  ││
│  │ • Cache hit rate: 94%                                       ││
│  │ • Regional coverage: NA, EU, APAC                           ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**UI Indicators**:
- Tier badge (0-3) with color
- Replication health bar
- Peer count and geographic map
- CDN coverage indicator

---

## Implementation Roadmap

### Phase 1: Complete CRUD Foundation

**Backend (elohim-storage)**:
- [ ] Add single-item POST for content/paths (not just bulk)
- [ ] Add PUT endpoints for content/paths (update metadata)
- [ ] Add DELETE endpoints for content/paths
- [ ] Add blob upload endpoint (multipart form)
- [ ] Add thumbnail processing (auto-resize, format conversion)

**Frontend (elohim-app)**:
- [ ] Extend StorageClientService with update/delete methods
- [ ] Add blob upload service with progress tracking
- [ ] Create ContentEditorService for managing drafts

### Phase 2: File Manager Component

**Core file manager features**:
- [ ] Grid/list view toggle
- [ ] Folder hierarchy (virtual, tag-based)
- [ ] Drag-and-drop upload
- [ ] Bulk operations (select, delete, move)
- [ ] Search and filter
- [ ] Preview pane

**Content type handlers**:
- [ ] Image preview with thumbnail generation
- [ ] Markdown preview/edit
- [ ] PDF viewer integration
- [ ] HTML5 app preview

### Phase 3: Dimension Indicators

**Safety dimension UI**:
- [ ] Encryption status badge
- [ ] Privacy level selector
- [ ] Key recovery options panel
- [ ] Share dialog with ACL management

**Reach dimension UI**:
- [ ] Concentric circles visualization
- [ ] Audience count display
- [ ] Sharing history timeline
- [ ] Community visibility settings

**Replication dimension UI**:
- [ ] Replication tier badge
- [ ] Health indicator (sync status)
- [ ] Peer map visualization
- [ ] CDN status dashboard

### Phase 4: Integration with Seeding Pipeline

**Thumbnail migration**:
- [ ] Update seeder to upload thumbnails as blobs
- [ ] Set `thumbnailBlobHash` instead of `thumbnailUrl`
- [ ] Remove static images from `public/images/`
- [ ] Update path JSON schema

**Content body migration**:
- [ ] Ensure all large content uses blob pattern
- [ ] Sparse storage for all content > 10KB
- [ ] Automatic blob extraction during seed

---

## Data Model Extensions

### StoragePath additions

```typescript
interface StoragePath {
  // Existing fields...

  // Safety
  visibility: 'private' | 'unlisted' | 'public';
  encrypted: boolean;
  encryptionKeyId?: string;

  // Reach
  reachLevel: 'personal' | 'household' | 'support' | 'community' | 'commons';
  sharedWith: string[];  // Agent IDs with explicit access

  // Replication
  replicationTier: 0 | 1 | 2 | 3;
  peerReplicaCount: number;
  shardDistribution?: {
    dataShards: number;
    parityShards: number;
    regions: string[];
  };
  cdnStatus?: {
    edgeNodes: number;
    cacheHitRate: number;
    regions: string[];
  };
}
```

### StorageContentNode additions

```typescript
interface StorageContentNode {
  // Existing fields...

  // Safety
  visibility: 'private' | 'unlisted' | 'public';
  encrypted: boolean;
  passwordProtected: boolean;

  // Reach (inherited from path or set directly)
  reachLevel: 'personal' | 'household' | 'support' | 'community' | 'commons';
  audienceCount: number;  // Computed from reach graph

  // Replication
  replicationTier: 0 | 1 | 2 | 3;
  syncStatus: 'syncing' | 'synced' | 'conflict' | 'offline';
  lastSyncedAt: string;
  replicaHealth: number;  // 0.0-1.0
}
```

---

## API Endpoints Needed

### Content CRUD

```
POST   /db/content              Create single content item
PUT    /db/content/:id          Update content metadata
DELETE /db/content/:id          Delete content (and blob if orphaned)
PATCH  /db/content/:id/reach    Update reach settings
PATCH  /db/content/:id/safety   Update safety settings
```

### Path CRUD

```
POST   /db/paths                Create single path
PUT    /db/paths/:id            Update path metadata
DELETE /db/paths/:id            Delete path
PATCH  /db/paths/:id/thumbnail  Upload/update thumbnail
```

### Blob Management

```
POST   /blob/upload             Upload blob (multipart)
POST   /blob/upload-thumbnail   Upload + auto-resize thumbnail
GET    /blob/:hash/metadata     Get blob metadata (size, type, replicas)
DELETE /blob/:hash              Delete blob (if no references)
```

### Replication Status

```
GET    /replication/:hash       Get replication status for blob
GET    /replication/stats       Get overall replication stats
POST   /replication/:hash/pin   Request increased replication
```

---

## Open Questions

1. **Encryption key management**: Where are encryption keys stored? Device keychain? Doorway? Support network threshold shares?

2. **Reach graph computation**: Is audience count computed live or cached? How often refreshed?

3. **CDN integration**: Is this via doorway infrastructure or external CDN? How does social-reach power edge caching?

4. **Conflict resolution**: When replication tier 2+ has conflicts, what's the resolution strategy?

5. **Quota management**: Are there storage limits per user? How are they enforced?

6. **Garbage collection**: When content is deleted, how are orphaned blobs cleaned up across the P2P network?

---

## Related Files

### Backend (elohim-storage)
- `elohim-storage/src/handlers/` - HTTP handlers for CRUD
- `elohim-storage/src/models/` - Diesel models
- `elohim-storage/src/blob_store.rs` - Blob storage logic

### Frontend (elohim-app)
- `elohim-app/src/app/elohim/services/storage-client.service.ts` - Storage client
- `elohim-app/src/app/elohim/services/storage-api.service.ts` - API service
- `elohim-app/src/app/elohim/services/content.service.ts` - Content loading

### Seeding
- `genesis/seeder/src/` - Content seeding pipeline
- `genesis/data/lamad/paths/` - Path JSON definitions

---

## Immediate Next Step

The quickest win to validate the architecture:

1. **Add `thumbnailBlobHash` support to seeder**
   - Upload thumbnail images as blobs during seed
   - Set `thumbnailBlobHash` on path records
   - Frontend already supports this via `getPathThumbnailUrl()`

2. **Test with hREA path**
   - The hREA logo at `genesis/docs/content/rea/hrea-logo.png` becomes the test case
   - Verify it loads from blob storage in alpha environment

This proves the blob-stored asset pattern before building the full file manager.
