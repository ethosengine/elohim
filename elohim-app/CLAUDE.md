# Elohim App - Angular Frontend

Angular 19 application for the Elohim learning platform. Connects to elohim-storage via doorway proxy or directly.

## Deployment Contexts

The app runs in three deployment modes with different content loading paths:

```
┌─────────────────────────────────────────────────────────────────┐
│                    DEPLOYMENT CONTEXTS                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Eclipse Che (Development)                                   │
│     Browser → Angular Dev Server (4200)                         │
│            → proxy.conf.mjs → localhost:8888                    │
│            → Doorway → elohim-storage                           │
│                                                                 │
│  2. Local Development                                           │
│     Browser → Angular Dev Server (4200)                         │
│            → proxy.conf.mjs → localhost:8888                    │
│            → Doorway → elohim-storage                           │
│                                                                 │
│  3. Production / Alpha                                          │
│     Browser → doorway.host (HTTPS)                              │
│            → Doorway → elohim-storage                           │
│                                                                 │
│  4. Tauri Desktop                                               │
│     App → localhost:8090 (direct to elohim-storage sidecar)     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Eclipse Che Specifics

The Che dev-proxy **strips CORS headers** from responses, causing issues with cross-origin requests. Solution:

1. Route all API requests through Angular's dev server proxy (same-origin)
2. `doorway-connection-strategy.ts` returns `window.location.origin` for Che environment
3. All `/api/*`, `/db/*`, `/blob/*`, `/apps/*` routes proxy to doorway

```typescript
// In doorway-connection-strategy.ts
if (this.isCheEnvironment() && config.useLocalProxy) {
  return window.location.origin;  // Same-origin avoids CORS
}
```

---

## Content Loading Flow

Content can be stored in two patterns:

### 1. Inline Content
Content body stored directly in the `contentBody` field:
```json
{
  "id": "concept-123",
  "contentBody": "# Markdown content here...",
  "contentFormat": "markdown"
}
```

### 2. Sparse/Blob Pattern
Large content stored as blob, reference in `contentBody`:
```json
{
  "id": "article-456",
  "contentBody": "sha256-abc123...",
  "blobCid": "sha256-abc123...",
  "contentFormat": "markdown"
}
```

The `ContentService` automatically detects blob references and fetches:
```typescript
// content.service.ts
const isBlobReference = contentBody.startsWith('sha256:') || contentBody.startsWith('sha256-');
if (isBlobReference) {
  return this.fetchBlobContent(contentBody);
}
```

### 3. HTML5 App Content
Interactive apps (like Evolution of Trust) store metadata object:
```json
{
  "id": "simulation-evolution-of-trust",
  "contentBody": {
    "appId": "evolution-of-trust",
    "entryPoint": "index.html",
    "fallbackUrl": "https://..."
  },
  "contentFormat": "html5-app"
}
```

The app is served from `/apps/{appId}/{entryPoint}` via doorway, which extracts files from the ZIP blob.

---

## Proxy Configuration

### proxy.conf.mjs (Angular 19 ESM format)
```javascript
export default [
  {
    context: ['/api', '/db', '/blob', '/apps', '/health'],
    target: 'http://localhost:8888',
    secure: false,
    changeOrigin: true,
  },
];
```

### Key Routes
| Route | Purpose |
|-------|---------|
| `/db/content/*` | Content CRUD |
| `/db/paths/*` | Learning paths |
| `/blob/*` | Raw blob storage |
| `/apps/*` | HTML5 app serving |
| `/api/v1/cache/*` | Doorway projection cache |

---

## Debugging Content Loading

### Quick Verification
```bash
# Check proxy forwarding
curl http://localhost:4200/db/content/quiz-manifesto-foundations | jq

# Check doorway directly
curl http://localhost:8888/db/content/quiz-manifesto-foundations | jq

# Check blob content
curl http://localhost:8888/blob/sha256-abc123...
```

### Common Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| CORS errors | Che proxy stripping headers | Use Angular proxy (same-origin) |
| Content shows `sha256:...` | Blob reference not resolved | Check `fetchBlobContent` in content.service.ts |
| Thumbnail not loading | Relative URL needs base | Check `getStorageBaseUrl()` returns origin |
| HTML5 app shows metadata | Sparse pattern used for html5-app | Ensure html5-app keeps original content object |

### Debug Logs
```typescript
// Enable in content.service.ts
console.debug('[ContentService] Fetching blob from:', blobUrl);

// Enable in sophia-renderer.component.ts
console.log('[SophiaRenderer] loadMoments:', { nodeId, contentFormat });
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/app/elohim/services/content.service.ts` | Content fetching, blob resolution |
| `src/app/elohim/services/storage-client.service.ts` | Storage API client, URL construction |
| `proxy.conf.mjs` | Angular dev server proxy config |
| `elohim-library/.../doorway-connection-strategy.ts` | Deployment context detection |

---

## Content Formats

| Format | Renderer | Notes |
|--------|----------|-------|
| `markdown` | MarkdownRendererComponent | Standard content |
| `sophia`, `sophia-quiz-json` | SophiaRendererComponent | Quiz/assessment |
| `perseus`, `perseus-quiz-json` | PerseusRendererComponent | Legacy quiz format |
| `html5-app` | IframeRendererComponent | Interactive apps from ZIP |
| `html`, `text` | Basic renderers | Simple content |

---

## Starting Development

```bash
# Start with seeding (recommended for fresh start)
npm run hc:start:seed

# Start without seeding (if data exists)
npm run hc:start

# Angular dev server only (if doorway already running)
npm start
```
