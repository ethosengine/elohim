---
name: holochain-storage-api
description: Reference for elohim-storage HTTP API, storage-client SDK, type generation pipeline, and Angular adapter conventions. Use when someone asks "how do I call the storage API", "fetch content or paths", "upload a blob", "create mastery records", "generate TypeScript types from Rust", or works with the Rust-to-TypeScript type boundary.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# Holochain Storage API Reference

This skill covers the elohim-storage HTTP API, the `@elohim/storage-client` TypeScript SDK, the Rust-to-TypeScript type generation pipeline, and Angular adapter conventions.

## Architecture

```
Browser / Tauri App
       |
       v
Angular App (localhost:4200)
       |
       v (proxy.conf.mjs or direct)
Doorway Gateway (localhost:8888)       <-- proxies /db/*, /blob/*, /apps/*
       |
       v
elohim-storage (localhost:8090)        <-- SQLite content DB + blob store
       |
       v
SQLite (content, paths, mastery, economic events, sessions)
+ Filesystem (blob shards, Reed-Solomon 4+3)
```

### Two Access Modes (Same HTTP API)

| Mode | Path | Use Case |
|------|------|----------|
| **Doorway proxy** | Browser -> doorway:8888 -> storage:8090 | Web (dev + prod) |
| **Direct** | Tauri -> localhost:8090 | Desktop app (sidecar) |

Both modes hit the same `http.rs` routes. No FFI, no direct SQLite from TypeScript.

---

## HTTP API Endpoints

All endpoints return **camelCase JSON**. All JSON fields are pre-parsed (no `JSON.parse()` needed).

Key endpoint groups: `/db/content`, `/db/paths`, `/db/mastery`, `/db/economic-events`, `/db/contributor-presences`, `/db/stewardship-allocations`, `/blob/`, `/session`, `/health`.

See `references/endpoints.md` for full endpoint tables, View/InputView type listings, and the Rust transformation pattern.

---

## `@elohim/storage-client` SDK

The TypeScript SDK at `holochain/sdk/storage-client-ts/`.

### Setup

```typescript
import { StorageClient } from '@elohim/storage-client';

const client = new StorageClient({
  baseUrl: 'http://localhost:8090',
  appId: 'lamad',
  apiKey: 'optional-bearer-token',
  timeout: 30000,  // default
});
```

### Sync API Methods

```typescript
// List documents
const { documents, total } = await client.listDocuments({ prefix: 'graph', limit: 10 });

// Get document info
const doc = await client.getDocument('graph:my-doc');

// Get document heads (Automerge)
const { heads } = await client.getHeads('graph:my-doc');

// Get changes since known heads
const { changes, new_heads } = await client.getChangesSince('graph:my-doc', knownHeads);

// Apply changes
await client.applyChanges('graph:my-doc', changesAsUint8Arrays);

// Count documents
const count = await client.countDocuments();
```

### Blob API Methods

```typescript
// Upload blob
const manifest = await client.putBlob(data, 'image/png');

// Get blob
const data: Uint8Array = await client.getBlob('sha256-abc123');

// Check existence
const exists: boolean = await client.blobExists('sha256-abc123');

// Get manifest
const manifest = await client.getManifest('sha256-abc123');
```

---

## Type Generation Pipeline

Types flow from Rust to TypeScript automatically:

```
views.rs (Rust View types)
    |  #[derive(TS)]
    |  #[serde(rename_all = "camelCase")]
    |  #[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
    v
cargo test export_bindings
    |
    v
holochain/sdk/storage-client-ts/src/generated/*.ts
    |
    v
@elohim/storage-client (npm package)
    |
    v
Angular app imports: import { ContentView } from '@elohim/storage-client';
```

### Regenerating Types

```bash
cd holochain/elohim-storage
cargo test export_bindings

cd ../sdk/storage-client-ts
npm run build
```

Key View types: `ContentView`, `PathWithDetailsView`, `ContentMasteryView`, `EconomicEventView`, `ContributorPresenceView`, `StewardshipAllocationView`. Key InputView types: `CreateContentInputView`, `CreatePathInputView`, `CreateMasteryInputView`, `CreateEconomicEventInputView`.

See `references/endpoints.md` for the full type listing and the Rust `From` transformation pattern.

---

## Angular Adapter Conventions

The adapter layer at `elohim-app/src/app/elohim/adapters/` follows strict rules:

### Rules

1. **Re-export only** - Adapters re-export generated types, never transform
2. **No `JSON.parse()`** - All JSON is pre-parsed by Rust views
3. **No case conversion** - All fields arrive as camelCase
4. **No `toWire`/`fromWire`** - No transformation functions
5. **Computed fields only** - Adapters may add derived/computed properties

### Import Pattern

```typescript
// From generated types (preferred)
import { ContentView, PathWithDetailsView } from '@elohim/storage-client';

// Via adapter (when computed fields needed)
import { StorageTypesAdapter } from '@app/elohim';
```

---

## Content Patterns

### Inline Content

Content body stored directly:
```json
{
  "id": "concept-123",
  "contentBody": "# My Content\n\nMarkdown here...",
  "contentFormat": "markdown"
}
```

### Sparse/Blob Pattern

Large content stored as blob reference:
```json
{
  "id": "article-456",
  "contentBody": "sha256-abc123...",
  "blobCid": "sha256-abc123...",
  "contentFormat": "markdown"
}
```

The `ContentService` auto-detects blob references (`sha256:` or `sha256-` prefix) and fetches.

### HTML5 App Content

Interactive apps stored as metadata + ZIP blob:
```json
{
  "id": "simulation-evolution-of-trust",
  "contentBody": { "appId": "evolution-of-trust", "entryPoint": "index.html" },
  "contentFormat": "html5-app"
}
```

Served from `/apps/{appId}/{entryPoint}` via doorway (ZIP extraction).

---

## Common Operations

### Read Content and Render

```typescript
// In Angular service
const content: ContentView = await this.http.get<ContentView>(
  `${this.baseUrl}/db/content/${id}`
).toPromise();

// Content is ready to use - no parsing needed
console.log(content.contentType);  // string
console.log(content.metadata);     // object | null
```

### Write Mastery

```typescript
const input: CreateMasteryInputView = {
  humanId: currentUser.id,
  contentId: 'concept-123',
  masteryLevel: 'familiar',
};
await this.http.post(`${this.baseUrl}/db/mastery`, input).toPromise();
```

### Create Learning Path

```typescript
const input: CreatePathInputView = {
  id: 'path-governance',
  title: 'Governance Fundamentals',
  chapters: [{
    id: 'ch-1',
    title: 'Introduction',
    steps: [{
      id: 'step-1',
      pathId: 'path-governance',
      title: 'What is Governance?',
      resourceId: 'concept-governance-intro',
      resourceType: 'content',
    }],
  }],
};
```

### Upload and Reference Blob

```typescript
// Upload
const response = await fetch(`${baseUrl}/blob/`, {
  method: 'PUT',
  headers: { 'Content-Type': 'image/png' },
  body: imageData,
});
const manifest = await response.json();

// Reference in content
const content: CreateContentInputView = {
  id: 'image-123',
  title: 'Architecture Diagram',
  blobHash: manifest.hash,
  blobCid: manifest.cid,
  contentFormat: 'image',
};
```

---

## Schema Versioning

All InputView types include `schemaVersion` (defaults to 1):

```typescript
const input: CreateContentInputView = {
  id: 'test',
  title: 'Test',
  schemaVersion: 1,  // optional, defaults to 1
};
```

Server validates against `SUPPORTED_SCHEMA_VERSIONS`. Unknown fields are silently ignored (tolerant reader pattern).

---

## Gotchas

1. **RUSTFLAGS override required** for building elohim-storage:
   ```bash
   RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
   ```

2. **Proxy vs direct**: Dev uses Angular proxy (`:4200` -> `:8888` -> `:8090`). Tauri hits `:8090` directly. Same API.

3. **Never `JSON.parse()` in TypeScript** - Metadata, provenance chains, evidence, etc. arrive pre-parsed as `Value` / `JsonValue`.

4. **Boolean coercion** - SQLite stores bools as `INTEGER (0/1)`. Views convert to proper `bool`. TypeScript receives `boolean`.

5. **Sparse vs inline** - Check `contentBody` for `sha256:` prefix to detect blob references.

6. **Tauri mode** - No doorway proxy. Direct HTTP to `localhost:8090`. Same endpoints.

7. **`app_id` scoping** - Multi-tenant: all queries are scoped by `app_id`. The SDK sets this via `StorageConfig.appId`.

---

## Key Files

| File | Purpose |
|------|---------|
| `holochain/elohim-storage/src/views.rs` | API boundary - all View/InputView types |
| `holochain/elohim-storage/src/http.rs` | HTTP route handlers |
| `holochain/elohim-storage/CLAUDE.md` | Detailed boundary architecture guide |
| `holochain/sdk/storage-client-ts/src/client.ts` | TypeScript SDK client |
| `holochain/sdk/storage-client-ts/src/generated/` | Auto-generated TypeScript types |
| `holochain/sdk/storage-client-ts/CLAUDE.md` | Generated types usage guide |
| `holochain/ARCHITECTURE.md` | Overall architecture |
| `holochain/P2P-DATAPLANE.md` | P2P data plane design |
| `elohim-app/src/app/elohim/adapters/storage-types.adapter.ts` | Angular adapter layer |

## External References

- Holochain Developer Docs: `https://developer.holochain.org/`
- HDK API (0.6): `https://docs.rs/hdk/0.6.0/hdk/`
- ts-rs (Rust -> TypeScript): `https://docs.rs/ts-rs/`
