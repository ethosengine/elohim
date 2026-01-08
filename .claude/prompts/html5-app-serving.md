# HTML5 App Serving Route Implementation

## Goal
Implement server-side rendering for HTML5 applications, serving extracted ZIP content from blob storage via `/apps/{appId}/{path}` routes. Must work in both web and native Tauri modes.

## Architecture

**Doorway** = Projection layer (read-only view cache from P2P content-server layer, includes both DHT and storage backends)
**elohim-storage** = Backend storage layer (SQLite metadata, blob storage, P2P node)

### P2P Infrastructure (Existing)
The P2P layer in `holochain/elohim-storage/src/p2p/` provides:
- **ShardProtocol** (`/elohim/shard/1.0.0`) - Blob shard transfer between nodes
- **SyncProtocol** (`/elohim/sync/1.0.0`) - CRDT document synchronization
- **Kademlia DHT** - Content routing and discovery
- **mDNS** - Local network peer discovery

Blobs can sync from other devices via ShardProtocol - this is the native P2P experience.

### Deployment Modes

| Mode | App Serving | P2P |
|------|-------------|-----|
| **Web** | Browser → Doorway `/apps/` → elohim-storage | Client connects to hosted Doorway |
| **Native (Tauri)** | Tauri → Embedded P2P node `/apps/` | Direct P2P participation, blobs sync from peers |

The `/apps/` route serves from local P2P cache - blobs may originate from seeded content or sync from other devices on the network.

## Context

### Current State
- HTML5 app metadata stored inline in SQLite (`content_body`)
- ZIP files exist in `genesis/docs/content/fct/` (e.g., `evolution-of-trust.zip` - 7MB)
- Content JSON references ZIP via `metadata.localZipPath`
- Angular `IframeRenderer` expects URL: `${baseUrl}/apps/${appId}/${entryPoint}`
- Currently falls back to `fallbackUrl` since `/apps/` route doesn't exist

### Content Structure
```json
{
  "id": "simulation-evolution-of-trust",
  "contentFormat": "html5-app",
  "content": {
    "appId": "evolution-of-trust",
    "entryPoint": "index.html",
    "fallbackUrl": "https://ncase.me/trust/"
  },
  "metadata": {
    "localZipPath": "docs/content/fct/evolution-of-trust.zip",
    "embedStrategy": "iframe",
    "securityPolicy": {
      "sandbox": ["allow-scripts", "allow-same-origin"],
      "csp": "default-src 'self'; script-src 'unsafe-inline'"
    }
  }
}
```

## Implementation Tasks

### 1. Blob Storage for ZIPs (elohim-storage)
Update seeder to upload ZIP files:
- Read ZIP from `metadata.localZipPath`
- Upload to blob storage via `/blob/` endpoint
- Store returned `blob_hash` in content record
- Track `blob_cid` for IPFS compatibility
- ZIP becomes available to P2P network via ShardProtocol

### 2. Doorway `/apps/` Route (Web Mode)
Create `holochain/doorway/src/routes/apps.rs`:

```rust
// Route: GET /apps/{app_id}/{path:.*}
// 1. Look up content by appId (via storage backend)
// 2. Get blob_hash from content record
// 3. Fetch ZIP from elohim-storage blob endpoint
// 4. Extract requested file from ZIP (with caching)
// 5. Serve with appropriate Content-Type
```

### 3. Native P2P App Serving (Tauri Mode)
For native apps, the embedded P2P node serves `/apps/` route:
- Same route logic as Doorway
- Blobs sourced from local cache or fetched via ShardProtocol from peers
- ZIP extraction with same caching strategy
- Tauri registers custom protocol handler that routes to embedded HTTP server

```typescript
// IframeRenderer URL construction
const baseUrl = this.environment.native
  ? 'tauri://localhost'  // Routes to embedded P2P node
  : this.environment.doorwayUrl;
const appUrl = `${baseUrl}/apps/${appId}/${entryPoint}`;
```

### 4. ZIP Extraction Service (Shared)
Create extraction service used by both Doorway and native node:
- In-memory ZIP extraction using `zip` crate
- LRU cache for extracted apps (configurable size limit)
- Lazy extraction: only extract files as requested
- Full extraction option for frequently accessed apps

### 5. Content-Type Detection
Map file extensions to MIME types:
```rust
".html" => "text/html"
".js" => "application/javascript"
".css" => "text/css"
".png" => "image/png"
".svg" => "image/svg+xml"
".json" => "application/json"
".wasm" => "application/wasm"
```

## File Structure
```
holochain/doorway/src/
├── routes/
│   ├── mod.rs          # Add apps module
│   └── apps.rs         # NEW: /apps/ route handler
└── services/
    ├── mod.rs          # Add app_extractor module
    └── app_extractor.rs # NEW: ZIP extraction + caching (could be shared crate)

holochain/elohim-storage/src/
├── p2p/
│   ├── mod.rs          # Existing P2P node
│   └── shard_protocol.rs # ShardRequest::Get for blob fetch
└── routes/
    └── apps.rs         # Same /apps/ logic for native serving
```

## API Design

### GET /apps/{app_id}/{path}
- **200**: File content with appropriate Content-Type
- **404**: `{ "error": "not_found", "fallback": "https://..." }`
- **500**: Extraction or blob fetch error

### Cache Headers
```
Cache-Control: public, max-age=31536000, immutable
ETag: {blob_hash}-{file_path_hash}
```

## Security Considerations
- Path traversal prevention (no `..` in paths)
- Content-Security-Policy from metadata
- Sandbox attributes for iframe embedding
- No execution of server-side code from ZIPs
- P2P: Verify blob hash matches expected content

## Testing
1. **Web**: Upload ZIP to blob storage, request via Doorway
2. **Native**: Start P2P node, verify blob syncs from peer, serve via embedded route
3. **Offline**: Pre-seed blob locally, verify app loads without network
4. Verify game loads in iframe in all modes

## Dependencies
- `zip` crate for extraction
- `mime_guess` crate for Content-Type detection
- Existing P2P infrastructure (ShardProtocol, BlobStore)
- Tauri custom protocol handler → embedded HTTP server

## Related Files
- `elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts`
- `elohim-app/src/app/lamad/content-io/plugins/html5-app/html5-app-format.plugin.ts`
- `genesis/data/lamad/content/simulation-evolution-of-trust.json`
- `genesis/docs/content/fct/evolution-of-trust.zip`
- `holochain/doorway/src/routes/mod.rs`
- `holochain/elohim-storage/src/p2p/mod.rs` - P2P node with ShardProtocol
- `holochain/elohim-storage/src/p2p/shard_protocol.rs` - Blob sync protocol
- `holochain/elohim-storage/src/blob_store.rs`
