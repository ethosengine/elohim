# Elohim App Architecture

This document explains the progressive sovereignty architecture and data flow patterns used in elohim-app.

## Progressive Sovereignty Model

The Elohim platform supports multiple deployment modes with increasing levels of user sovereignty:

| Mode | Platform | Data Access | Offline | Sovereignty Level |
|------|----------|-------------|---------|-------------------|
| **Browser/Doorway** | Web app | Via doorway proxy | No | Hosted (doorway manages keys) |
| **Tauri/Direct** | Desktop app | Local elohim-storage | Full | Full (local SQLite + keys) |
| **Federation Node** | Server | DHT participation | Full | Full + network contribution |

## Unified API Boundary

Both Doorway (hosted) and Tauri (native) use the **same** elohim-storage HTTP API:

```
                    ┌─────────────────────────────────────┐
                    │         elohim-storage              │
                    │   (http.rs / views.rs unified API)  │
                    │                                     │
                    │  /db/content, /session, /store/...  │
                    │         camelCase boundary          │
                    │              ↓                      │
                    │           SQLite                    │
                    └─────────────────────────────────────┘
                                    ▲
                    ┌───────────────┴───────────────┐
                    │                               │
            ┌───────┴───────┐             ┌────────┴────────┐
            │   Doorway     │             │  elohim-storage │
            │  (proxy at    │             │  sidecar at     │
            │  doorway.host)│             │  localhost:8090 │
            └───────┬───────┘             └────────┬────────┘
                    │                               │
                    ▼                               ▼
            ┌───────────────┐             ┌─────────────────┐
            │    Browser    │             │  Tauri App      │
            │ (hosted human)│             │ (sovereign user)│
            └───────────────┘             └─────────────────┘
```

**Key Insight**: Tauri does NOT have direct SQLite bindings or Rust FFI. It uses standard HTTP fetch to the same API endpoints, just at `localhost:8090` instead of through doorway.

## Service Layer Stack

```
┌──────────────────────────────────────────────────────────┐
│                    DataLoaderService                      │
│           (single abstraction - all data sources)         │
│                                                          │
│  • checkReadiness() - lightweight connectivity check     │
│  • getContent(id) - single item with caching             │
│  • getContentIndex() - bulk metadata for search/browse   │
│  • getPath(id) - learning path with steps                │
└──────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
     ┌─────────────────┐         ┌─────────────────┐
     │  ContentService │         │ StorageApiService│
     │ (via ElohimClient)        │ (domain queries) │
     │                 │         │                 │
     │ Routes by mode: │         │ • relationships │
     │ • doorway→proxy │         │ • presences     │
     │ • tauri→local   │         │ • events        │
     └─────────────────┘         └─────────────────┘
              │                           │
              └───────────┬───────────────┘
                          ▼
                 ┌─────────────────┐
                 │  elohim-storage │
                 │   http.rs API   │
                 │  (unified for   │
                 │   all clients)  │
                 └─────────────────┘
```

## Connection Strategy Pattern

The app uses a strategy pattern to handle different deployment modes:

```typescript
// Mode detection (in elohim-service/src/connection/)
function detectConnectionMode(): ConnectionMode {
  if (typeof __TAURI__ !== 'undefined') return 'tauri';
  if (typeof process !== 'undefined') return 'direct';
  return 'doorway';  // Browser default
}
```

### Strategies:

1. **DoorwayConnectionStrategy** (Browser)
   - WebSocket through `wss://doorway.host/hc/app/{port}`
   - HTTP through `https://doorway.host/api/...`
   - Projection cache for fast reads

2. **TauriConnectionStrategy** (Desktop)
   - Direct WebSocket to `ws://localhost:{port}`
   - HTTP to local `http://localhost:8090/...`
   - Full offline capability

3. **DirectConnectionStrategy** (Node.js/CLI)
   - Direct conductor connection
   - Used for tooling and scripts

## ContentService vs ProjectionAPIService

These services complement each other:

| Service | Purpose | When to Use |
|---------|---------|-------------|
| **ContentService** | Mode-aware content operations via ElohimClient | Primary content access (read/write) |
| **ProjectionAPIService** | Direct HTTP to doorway projection cache | Legacy pattern, browser-only fast reads |
| **StorageApiService** | Domain-specific queries (relationships, presences) | Graph queries, social data |

**ContentService** routes through ElohimClient which picks the right backend:
- Browser → doorway projection API (`/api/v1/cache/*`)
- Tauri → local elohim-storage (`http://localhost:8090/db/*`)

## Data Flow Examples

### Reading Content (Browser)
```
Component → DataLoaderService.getContent(id)
         → ContentService.getContent(id)
         → ElohimClient.get('content', id)
         → DoorwayConnectionStrategy
         → GET https://doorway.host/api/v1/cache/Content/{id}
         → Doorway projection cache (MongoDB)
         → Response with camelCase JSON
```

### Reading Content (Tauri)
```
Component → DataLoaderService.getContent(id)
         → ContentService.getContent(id)
         → ElohimClient.get('content', id)
         → TauriConnectionStrategy
         → GET http://localhost:8090/db/content/{id}
         → elohim-storage http.rs/views.rs
         → SQLite
         → Response with camelCase JSON
```

### Writing Content (Both Modes)
```
Component → ContentService.create(content)
         → ElohimClient.write('content', content)
         → WriteBuffer (batches with backpressure)
         → POST to appropriate endpoint
         → elohim-storage http.rs
         → SQLite + DHT sync
```

## Reach-Based Access Control

Content visibility is enforced through `ReachLevel`:

```
ReachLevel.Private       (agent-only)
ReachLevel.Invited       (invited group)
ReachLevel.Local         (bioregion)
ReachLevel.Neighborhood  (mutual aid network)
ReachLevel.Municipal     (municipality)
ReachLevel.Bioregional   (watershed/bioregion)
ReachLevel.Regional      (default authenticated)
ReachLevel.Commons       (public/anonymous)
```

The `ReachEnforcer` class checks if an agent's reach level permits access to content.

## Performance Considerations

1. **Readiness Check**: Use `DataLoaderService.checkReadiness()` for lightweight connectivity verification instead of `getContentIndex()` which loads all content.

2. **Caching**:
   - `shareReplay(1)` prevents redundant network calls
   - IndexedDB provides offline persistence
   - Projection cache (doorway) provides fast reads

3. **Lazy Loading**: Content index is loaded only when needed (search, browse), not on every page load.

## Related Documentation

- `elohim-storage/CLAUDE.md` - Unified HTTP API boundary
- `elohim-library/.../connection/CLAUDE.md` - Connection strategies
- `elohim-app/.../adapters/CLAUDE.md` - Type transformation patterns
