# Doorway SPA-as-Blob Design

**Date:** 2026-04-02
**Status:** Approved
**Trigger:** Alpha 502 cascade on `/apps/evolution-of-trust` exposed cross-origin gap preventing service worker ZIP delivery; 30+ individual file proxies from doorway to storage overwhelmed the pod on cold projection cache.

## Vision

Doorway becomes the single web2 ingress. One DNS record, one origin. The SPA is a ZIP blob in the content system, served by doorway through the same extraction cache mechanism used for HTML5 apps. No nginx SPA container. No second origin. No CORS.

**Arc:** B (SPA-as-blob, now) → C (SPA-as-EPR, future — the front door is a composite EPR materialized from the knowledge graph).

**SDK promise:** "Point DNS at your doorway" is the entire web2 deployment story.

---

## 1. New Content Format: `spa-bundle`

A protocol-level format that declares "I am a doorway's root web2 surface."

**Protocol schema** (`elohim/sdk/schemas/v1/enums/content-format.schema.json`):
Add `"spa-bundle"` to the enum array.

**Lamad manifest** (`elohim/sdk/domains/lamad/manifest.json`):
```json
"spa-bundle": {
  "description": "Single-page application served as a doorway's root web surface. ZIP archive extracted and cached by doorway with SPA routing (unmatched paths serve index.html).",
  "renderer": null,
  "mimeTypes": ["application/zip"],
  "extensions": ["zip"]
}
```

`renderer: null` — the SPA doesn't render inside another app; it IS the app. This distinguishes it from `html5-app` (which renders in an iframe).

**Seed content node:**
```json
{
  "id": "lamad-spa",
  "contentType": "application",
  "contentFormat": "spa-bundle",
  "title": "Lamad Learning Platform",
  "content": {
    "slug": "lamad",
    "entryPoint": "index.html"
  },
  "blobHash": "<sha256 of Angular dist ZIP>",
  "reach": "commons"
}
```

---

## 2. Root App Resolution

Doorway resolves a **root app** — a `spa-bundle` content node identified by the `ROOT_APP_SLUG` environment variable (e.g., `lamad`).

### Fallback chain

```
No ROOT_APP_SLUG configured:
  / → 302 redirect to /threshold (operator dashboard)
      Operator can configure their doorway, register an SPA, manage humans.

ROOT_APP_SLUG configured, SPA not yet loaded:
  / → bootstrap page ("Connecting to the network...")
      Auto-refreshes until SPA blob is extracted.

ROOT_APP_SLUG configured, SPA loaded:
  / → SPA served from extraction cache
```

### Request routing (http.rs)

After all explicit API routes, before the `_ => 404` catch-all:

```
Request: GET /content/manifesto  (no API route matched)
  │
  ├─ ROOT_APP_SLUG not set → redirect to /threshold
  │
  ├─ slug_index lookup: ROOT_APP_SLUG → blob_hash
  │   └─ not resolved → serve bootstrap page
  │
  ├─ extraction_cache check: is the SPA ZIP extracted?
  │   ├─ NO → fetch ZIP from blob store, extract, cache, then serve
  │   └─ YES → check if request path matches a real file
  │       ├─ file exists → serve it (with cache headers)
  │       └─ no match   → serve index.html (SPA fallback)
  │
  └─ blob store fetch fails → serve bootstrap page
```

### Static asset vs SPA route detection

The root app handler checks the extraction cache for a file matching the request path. If found, serve it (static asset). If not found, serve `index.html` (SPA route). This means:

- `/main.abc123.js` → found in cache → serve JS file
- `/assets/logo.png` → found in cache → serve image
- `/content/manifesto` → not in cache → serve `index.html` (Angular router handles it)
- `/nonexistent.js` → not in cache → serve `index.html` (Angular shows 404 view)

No extension-based heuristic needed. The extraction cache is the authority — if the file exists in the ZIP, it's a static asset.

### Key differences from `/apps/{slug}/{file}`

| Aspect | `/apps/{slug}/{file}` | Root app |
|--------|----------------------|----------|
| URL prefix | `/apps/{slug}/` stripped | None — request path IS the file path |
| Unmatched paths | 404 | SPA fallback → `index.html` |
| Multiplicity | Many slugs, one per HTML5 app | One root app per doorway |
| Rendering context | iframe (embedded) | Full page (the application) |

### Paths that must NOT fall through to root app

All existing explicit routes continue to match first:
- `/db/*`, `/api/*`, `/blob/*`, `/apps/*`, `/auth/*`
- `/bootstrap`, `/signal`, `/threshold/*`
- `/health`, `/health/startup`
- `/.well-known/*`

Only truly unmatched GET requests fall through to root app resolution.

---

## 3. Bootstrap Page with Live Status

Embedded in the doorway binary as a `const &str`. Zero external dependencies.

### Behavior

1. Renders immediately with doorway identity (from config — no network call needed).
2. Polls `GET /health/startup` every 2 seconds via inline `fetch()`.
3. Updates status lines as each subsystem becomes ready.
4. Auto-navigates to `/` when `rootApp.ready` becomes `true`.
5. Falls back to a simple 5-second `setTimeout` reload if `/health/startup` is unreachable.

### New endpoint: `GET /health/startup`

```json
{
  "identity": {
    "ready": true,
    "did": "did:web:alpha.elohim.host"
  },
  "storage": {
    "ready": true,
    "url": "http://elohim-matthew-alpha:8090"
  },
  "projection": {
    "ready": false,
    "content": 14,
    "humans": 3,
    "relationships": 8
  },
  "rootApp": {
    "ready": false,
    "slug": "lamad",
    "blobHash": null,
    "extracted": false
  }
}
```

### Display

```
Connecting to the Elohim Protocol...

[✓] Doorway identity: did:web:alpha.elohim.host
[✓] Storage sidecar connected
[○] Warming projection cache... (14 content, 3 humans)
[ ] Loading lamad application...
```

Each line teaches a first-time visitor how the system works. For SDK implementers, it's a diagnostic dashboard — if the SPA isn't loading, the page tells you exactly which step failed.

### When is it served?

Only when `ROOT_APP_SLUG` is configured AND the root app ZIP isn't extracted yet (slug not resolved, or extraction cache empty for that slug). Once the SPA is cached, the bootstrap page is never served again unless the cache is cleared or the blob hash changes.

---

## 4. Warmup Retry (RCA Fix)

`spawn_stream_task` in `warm_stream.rs` currently fires once with a 10-second delay. If storage is unreachable (e.g., pod restart timing), the projection cache stays empty permanently.

### Change

Add retry with exponential backoff per peer:

```
For each storage_url:
  attempt = 0
  loop:
    result = stream_from_peer(store, storage_url)
    if result has content OR no errors:
      break  // success (partial warmup is useful)
    attempt += 1
    if attempt >= 5:
      warn and break  // give up after ~5 minutes total
    sleep(min(10s * 2^attempt, 120s))
      // 10s → 20s → 40s → 80s → 120s
```

Generous backoff — this runs in the background, no one is waiting.

### Interaction with root app

The warmup populates the slug index, which resolves `ROOT_APP_SLUG → blob_hash`. Until warmup succeeds, the bootstrap page shows "Loading application..." as pending. The moment warmup resolves the slug, doorway extracts the ZIP, and the next poll/refresh loads the real SPA.

---

## 5. CI Pipeline Changes

### Current flow (Jenkinsfile root)

1. Build Angular → `dist/elohim-app/browser/`
2. Copy into nginx Docker image
3. Push to Harbor registry
4. Deploy as `elohim-site-alpha-service`

### New step (after step 1, before step 2)

```
1a. ZIP the dist directory
1b. Compute SHA256 hash
1c. Upload blob to storage: PUT {STORAGE_URL}/blob/{hash}
1d. Update content node: PUT {STORAGE_URL}/db/content/lamad-spa (with new blobHash)
```

Uses the same `StorageClient` / HTTP calls the seeder already uses. Targets storage directly (not doorway).

### Transition period (approach B)

Keep steps 2-4 during validation. The nginx container still serves `alpha.elohim.host` while the blob-served SPA is tested on `doorway-alpha.elohim.host`. Once proven, remove steps 2-4 and the nginx deployment.

---

## 6. Ingress Transition (Gradual)

### Phase 1 — Add `/apps/` and `/blob/` to main origin

```yaml
# alpha.elohim.host ingress — add before the catch-all
- path: /apps
  pathType: Prefix
  backend:
    service: elohim-doorway-alpha-service
    port:
      number: 8888
- path: /blob
  pathType: Prefix
  backend:
    service: elohim-doorway-alpha-service
    port:
      number: 8888
```

**Effect:** The apps-sw at `alpha.elohim.host` can now intercept `/apps/*` requests (same origin). ZIP delivery works on cold cache immediately. Nginx still serves the SPA at `/`.

### Phase 2 — Flip root to doorway

Replace the nginx backend with doorway for `/`:

```yaml
- path: /apps
  pathType: Prefix
  backend:
    service: elohim-doorway-alpha-service
    port:
      number: 8888
- path: /blob
  pathType: Prefix
  backend:
    service: elohim-doorway-alpha-service
    port:
      number: 8888
- path: /
  pathType: Prefix
  backend:
    service: elohim-doorway-alpha-service
    port:
      number: 8888
```

Delete `elohim-site-alpha-service` deployment and its nginx container.

### Phase 3 — End state (future)

Doorway handles TLS directly (ACME). No k8s ingress controller. DNS → doorway.

---

## 7. Iframe Renderer Change

`resolveDoorwayUrl()` in `iframe-renderer.component.ts` currently returns the full doorway URL (`https://doorway-alpha.elohim.host`) for production.

### Change

Return empty string for all environments:

```typescript
private resolveDoorwayUrl(): string {
  return '';
}
```

Everything is same-origin once ingress routes `/apps/` through the main domain. Che detection and local dev detection become unnecessary — behavior is identical everywhere.

---

## Files Changed

| File | Change |
|------|--------|
| `elohim/sdk/schemas/v1/enums/content-format.schema.json` | Add `spa-bundle` to enum |
| `elohim/sdk/domains/lamad/manifest.json` | Add `spa-bundle` format entry |
| `genesis/data/lamad/content/lamad-spa.json` | New seed content node |
| `doorway/doorway-service/src/server/http.rs` | Root app resolution after API routes; replace `/` redirect |
| `doorway/doorway-service/src/routes/root_app.rs` | New module: root app extraction + SPA fallback + bootstrap page |
| `doorway/doorway-service/src/routes/mod.rs` | Add `root_app` module |
| `doorway/doorway-service/src/routes/health.rs` | New `/health/startup` endpoint |
| `doorway/doorway-service/src/projection/warm_stream.rs` | Retry with exponential backoff |
| `doorway/doorway-service/src/config.rs` | `ROOT_APP_SLUG` env var |
| `doorway/doorway-service/src/main.rs` | Pass root app config to AppState |
| `app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts` | Return `''` from `resolveDoorwayUrl()` |
| `Jenkinsfile` (root) | Add ZIP + blob upload + content node update stage |
| `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml` | Phase 1: add `/apps`, `/blob` paths; Phase 2: flip `/` to doorway |

---

## Not In Scope

- **SPA-as-EPR (option C):** Future work. The composite EPR landing page that materializes from the knowledge graph. This design lays the foundation by making the SPA a content node.
- **Multi-app doorways:** One root app per doorway for now. `ROOT_APP_SLUG` is singular.
- **Doorway-native TLS/ACME:** Phase 3 of ingress transition. Not this sprint.
- **Removing `doorway-alpha.elohim.host`:** Kept for backwards compatibility. Both origins resolve to the same doorway.
