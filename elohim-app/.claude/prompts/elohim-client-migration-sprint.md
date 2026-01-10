# ElohimClient Migration Sprint

## Objective

Complete the migration from "abusing Holochain DHT as a database" to the proper architecture where:
- **Content** (heavy R/W): ElohimClient → Doorway → elohim-storage → SQLite/Postgres
- **Agent data** (Phase B): Holochain DHT for attestations, identity, points, consent

## Current State

### What's Done
- [x] `ElohimClient` TypeScript library created (`elohim-library/projects/elohim-service/src/client/`)
- [x] `ContentService` Angular service using ElohimClient (`elohim-app/src/app/elohim/services/content.service.ts`)
- [x] `provideElohimClient()` wired up in `app.config.ts`
- [x] Environment configs updated with `client` block (doorwayUrl, apiKey)
- [x] `@elohim/service` path alias configured in tsconfig
- [x] Genesis pipeline seeds to SQLite via doorway projection API

### What's Broken
The UI still uses the old DHT path:
```
UI → PathService → DataLoaderService → HolochainContentService → Zome calls → Empty DHT
```

Should be:
```
UI → PathService → DataLoaderService → ContentService → ElohimClient → Doorway → SQLite
```

## Migration Tasks

### 1. DataLoaderService Refactor
**File**: `elohim-app/src/app/elohim/services/data-loader.service.ts` (2400+ lines)

Replace HolochainContentService calls with ContentService calls:

| Old Method | New Method |
|------------|------------|
| `holochainContent.getPathIndex()` | `contentService.queryPaths()` |
| `holochainContent.getPath(id)` | `contentService.getPath(id)` |
| `holochainContent.getContent(id)` | `contentService.getContent(id)` |
| `holochainContent.batchGetContent(ids)` | `contentService.batchGetContent(ids)` |
| `holochainContent.searchContent(query)` | `contentService.searchContent(query)` |

**Key methods to update**:
- `getPathIndex()` - line 784
- `getPath()` - line 279
- `getContent()` - uses ContentResolver which may need update
- `batchGetContent()`
- Graph/relationship methods

### 2. ContentResolver Update
**File**: `elohim-app/src/app/elohim/services/content-resolver.service.ts`

The ContentResolver has tiered sources:
1. IndexedDB (local cache)
2. Projection API (doorway HTTP)
3. Conductor (Holochain zome calls) ← Remove this tier for content

Update to remove conductor as a content source. Conductor should only be used for agent-centric data.

### 3. App Startup Flow
**File**: `elohim-app/src/app/app.component.ts`

Current flow tests HolochainContentService availability. Update to:
1. Test doorway/projection API availability instead
2. Holochain connection becomes optional (for Phase B agent data)

### 4. Remove/Deprecate HolochainContentService for Content
**File**: `elohim-app/src/app/elohim/services/holochain-content.service.ts`

This service should only be used for:
- Agent-centric operations (attestations, identity)
- NOT for content/path retrieval

Mark content methods as deprecated or remove them.

## Architecture Reference

### Client Modes
```typescript
type ClientMode = BrowserMode | TauriMode;

// Browser: doorway-dependent, no offline
interface BrowserMode {
  type: 'browser';
  doorway: DoorwayConfig;
}

// Tauri: local SQLite, full offline
interface TauriMode {
  type: 'tauri';
  invoke: TauriInvoke;
  doorway?: DoorwayConfig;  // For sync
  nodes?: NodeSyncConfig;   // Personal nodes
}
```

### Data Flow
```
┌─────────────────────────────────────────────────────────────────┐
│                         Browser Mode                             │
├─────────────────────────────────────────────────────────────────┤
│  UI Component                                                    │
│       ↓                                                          │
│  ContentService (uses ElohimClient)                              │
│       ↓                                                          │
│  ElohimClient.get('content', id)                                 │
│       ↓                                                          │
│  HTTP GET https://doorway-dev.elohim.host/api/v1/content/{id}    │
│       ↓                                                          │
│  Doorway Projection Store (MongoDB cache of SQLite)              │
│       ↓                                                          │
│  elohim-storage SQLite (source of truth for content)             │
└─────────────────────────────────────────────────────────────────┘
```

### What Stays on Holochain (Phase B)
- Agent identity and keys
- Attestations (trust claims about humans/content)
- Points and participation metrics
- Consent relationships
- Presence signals

## Testing Checklist

- [ ] `/lamad` route loads paths from doorway projection
- [ ] Path detail pages load content from doorway
- [ ] Content search works via projection API
- [ ] No zome calls for content retrieval (check console logs)
- [ ] Holochain connection still works for agent data (if implemented)
- [ ] Offline mode degrades gracefully (shows cached content)

## Files to Modify

1. `elohim-app/src/app/elohim/services/data-loader.service.ts` - Main refactor
2. `elohim-app/src/app/elohim/services/content-resolver.service.ts` - Remove conductor tier
3. `elohim-app/src/app/app.component.ts` - Update startup flow
4. `elohim-app/src/app/elohim/services/holochain-content.service.ts` - Deprecate content methods

## Success Criteria

1. Console shows NO `[HolochainContent]` logs for content/path operations
2. Console shows `[ContentService]` or `[ElohimClient]` logs instead
3. Paths load from seeded SQLite data via doorway projection
4. App works when Holochain conductor is unavailable (content still loads)
