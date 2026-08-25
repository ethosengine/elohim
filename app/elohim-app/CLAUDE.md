---
id: elohim-app-frontend-gospel
cites:
  - "elohim-elements-ui-substrate-gospel | element/token/binding layer ownership — don't restyle elements from the shell | sha256:84cff1a46650cf8f | status: stale — target content moved on; re-verify | path: app/elohim-elements/CLAUDE.md"
  - "lamad-bundle-gospel | the bundle-consumer twin of the chrome & nav rails | sha256:1bc6eb8e1c112bc4 | status: stale — target content moved on; re-verify | path: app/lamad/CLAUDE.md"
  - "omnibar-consolidation-epr-native-links-design | settled decisions behind the chrome rails — serving context, sweep+interceptor, shared theme contract | sha256:92df16eea8d9bcf8 | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md"
---

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
2. `app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts` returns `window.location.origin` for Che environment
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
| `app/elohim-app/src/app/elohim/services/content.service.ts` | Content fetching, blob resolution |
| `app/elohim-app/src/app/elohim/services/storage-client.service.ts` | Storage API client, URL construction |
| `app/elohim-app/proxy.conf.mjs` | Angular dev server proxy config |
| `app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts` | Deployment context detection |

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

## Chrome & cross-bundle composition rails (2026-06-05)

The shell composes protocol chrome; it does not own element or token concerns.

- **protocol-omni is a trust surface** (`src/app/elohim/components/protocol-omni/`): EPR identity, resilience placeholder, opt-in ServingContext (`showEnvContext` — prod-silent, never cries wolf), opt-in theme toggle. Anything added here must be provenance-true, never decorative.
- **Theme**: `ThemeService` and elohim-core's `ThemeStore` are twins on ONE contract — `localStorage['elohim-theme']` + `html[data-theme]` (AUTHORITY — tokens.scss `:root[data-theme]` + `color-scheme` key off it; chrome var-chains substitute at `:root`) + `body[data-theme]` (legacy compat, dual-written) + the `elohim-theme-changed` event; each side adopts external changes silently, only the originator dispatches. Change the contract in both or neither.
- **Cross-bundle navigation**: never `routerLink`/`router.navigate` to another bundle's path (`/lamad*`). Template anchors → plain `href`; programmatic → `EprNavService.navigate()` (`ownsPath` derives from the live router config, so future pillar splits flip automatically; the sink refuses non-origin-relative targets); the capture-phase epr-link interceptor (explicit install in `app.component`) is the safety net for content-authored/legacy anchors.
- **Universal EPR address (§12.6 Slice 2, 2026-06-06)**: unclaimed EPR targets mint `/epr/{id}` via `eprToRoute(ref, BUNDLE_ROUTE_CONTEXT[, contentType])` — the shell provides `{ claims: [], ownsUniversalRoute: true }` and owns `epr/:resourceId` (cross-pillar viewer). `resource/:resourceId` stays as a legacy surface; new minting targets `/epr`. No pillar prefix in shared code.

Concern routing (content-addressed — resolve via this file's `cites:` frontmatter; slugs survive moves):
- `elohim-elements-ui-substrate-gospel` §Layer rails — element/token/binding layer ownership (don't restyle elements from the shell)
- `lamad-bundle-gospel` §EPR-app bundle rails — the bundle-consumer twin of these rails
- `omnibar-consolidation-epr-native-links-design` — the settled decisions behind these rails

## Starting Development

```bash
# Start with seeding (recommended for fresh start)
pnpm run hc:start:seed

# Start without seeding (if data exists)
pnpm run hc:start

# Angular dev server only (if doorway already running)
pnpm start
```

### Developer network profiles

| Profile | Command | What you get |
|---------|---------|--------------|
| `isolated` (default) | `pnpm run hc:start` | Full local stack on an island DHT — local conductor, storage, doorway; no external peers. |
| `live-data` | `pnpm start:alpha` | Local UI against deployed alpha data via the dev proxy (`proxy.conf.alpha.mjs`); HTTP contexts only — no local conductor in the data path. Read-mostly polish loops. |
| `join-alpha` | `NETWORK_PROFILE=join-alpha pnpm run hc:start` | Local conductor joins the alpha DHT via the deployed doorway's bootstrap+signal. DNA-hash parity is automatic: `scripts/fetch-deployed-dna.sh` fetches the DEPLOYED bundle (Harbor via oras, Jenkins artifact fallback) and the stack installs that instead of the local build. `FORCE_LOCAL_HAPP=1` keeps the local bundle (partition risk if hashes differ). Override endpoints with `CONDUCTOR_BOOTSTRAP_URL` / `CONDUCTOR_SIGNAL_URL`; pin the fetch with `DEPLOYED_HAPP_TAG` / `DEPLOYED_HAPP_BRANCH`. |

<!-- ci-marker(2026-05-24, retrigger after nexus PVC recovery): App pipeline rebuild after stageSpaBlob URL + 413 fixes.
     The orchestrator's graph-walker change-patterns for the App pipeline don't
     yet include genesis/orchestrator/manifests/elohim-app/** — so ingress changes
     under that path don't trigger an App build. This file touch forces the App
     pipeline to fire so stageSpaBlob can write fresh blob hashes into the
     elohim-host-landing + lamad-spa content rows. Once a follow-up shift adds
     manifests/elohim-app/** to the App pipeline's changePatterns, this marker
     becomes obsolete and can be removed. -->
