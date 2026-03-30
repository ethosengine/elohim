# Resilient HTML5 App Delivery — Sprint 2: Service Worker + Capability Negotiation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every browser/Tauri instance a self-sufficient peer with client-side extraction, and let peers advertise delivery capabilities so clients negotiate the best delivery mode.

**Architecture:** Two parts. (A) Extend the existing CapacityAnnouncement gossipsub with delivery capability strings and add a HEAD /_capability HTTP endpoint on both storage and doorway. (B) Register a raw Service Worker (not @angular/service-worker — more control, universal browser+Tauri) that intercepts /apps/ requests, caches files locally, and falls back to ZIP extraction when peer only serves compressed.

**Tech Stack:** Rust (elohim-storage identity.rs, http.rs; doorway apps.rs), TypeScript (raw Service Worker, Angular registration), JSZip (client-side extraction)

**Design:** `genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md`

**A2O Scenarios:**
- `genesis/a2o/features/delivery/client-resilience.feature` — 11 scenarios (SW registration, offline, capability negotiation, delivery modes)
- `genesis/a2o/features/federation/peer-advertisement.feature` — 16 scenarios (gossipsub heartbeat, neighbor table, dynamic state)
- `genesis/a2o/features/delivery/delivery-diagnostics.feature` — scenarios 7-11 (SW source reporting, layer disable, capability introspection)

**Sprint 1 Outcomes That Inform This Plan:**
- Doorway projection cache is live: `AppFileCacheService` with cache-first handler, coalescing, invalidation
- `X-Cache` headers already on all /apps/ responses (HIT/MISS/BYPASS/HIT-COALESCED)
- No existing Service Worker infrastructure in Angular app — clean slate
- `CapacityAnnouncement.capabilities` is `Vec<String>`, not a struct — delivery capabilities are string entries
- Route manifest system (`build_manifest()`) auto-discovers endpoints for doorway — HEAD endpoint must be declared there

---

## Part A: Capability Advertisement

### Task 1: DeliveryCapabilities struct + integration with NodeCapabilities

**Files:**
- Modify: `elohim/elohim-storage/src/identity.rs`

**What to build:**

Add a `DeliveryCapabilities` struct and `CacheTier` enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryCapabilities {
    /// Can serve individual extracted files from cache
    pub serves_extracted: bool,
    /// Can serve raw compressed blobs (client must extract)
    pub serves_compressed: bool,
    /// Content hashes this peer can serve file-by-file right now.
    /// Type-agnostic — infrastructure doesn't care what the content IS.
    pub ready_content: Vec<String>,
    /// Cache infrastructure tier
    pub cache_tier: CacheTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheTier {
    Projection,   // Doorway MongoDB — survives restarts, shared across replicas
    Extraction,   // Storage disk — device-local, budget-constrained
    BlobOnly,     // No cache — raw blob from blob store only
}
```

Add `delivery: DeliveryCapabilities` field to `NodeCapabilities`. Update the three preset profiles:
- `laptop()`: `serves_compressed: true, serves_extracted: false, cache_tier: BlobOnly, ready_content: vec![]`
- `home_node()`: `serves_compressed: true, serves_extracted: true, cache_tier: Extraction, ready_content: vec![]`
- `network_node()`: same as home_node but with larger readiness

Add a method `to_capability_strings(&self) -> Vec<String>` that converts DeliveryCapabilities into the string array format used by CapacityAnnouncement:
- `"serves_extracted"` if true
- `"serves_compressed"` if true
- `"cache_tier:extraction"` etc
- `"warm:{hash}"` for each entry in ready_content

**Tests:**
- Unit test for each preset profile's delivery capabilities
- Unit test for `to_capability_strings()` output format
- Unit test: `ready_content` entries produce `"warm:{hash}"` strings

**Verify:** `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Commit:** `feat(storage): add DeliveryCapabilities to NodeCapabilities`

---

### Task 2: ExtractionCache ready_content_hashes method

**Files:**
- Modify: `elohim/elohim-cache-core/src/extraction/cache.rs`

**What to build:**

Add a public method that returns the list of blob hashes currently warm in the extraction cache:

```rust
/// Returns blob hashes of all currently cached (non-expired) extractions.
/// Used for capability advertisement — tells peers which content
/// this node can serve file-by-file right now.
pub async fn ready_content_hashes(&self) -> Vec<String> {
    let index = self.index.read().await;
    index.values()
        .filter(|entry| !self.is_expired(entry))
        .map(|entry| entry.blob_hash.clone())
        .collect()
}
```

**Tests:**
- Unit test: empty cache returns empty vec
- Unit test: after put_app, hash appears in ready_content_hashes
- Unit test: expired entries are excluded

**Verify:** `cd elohim/elohim-cache-core && cargo test`

**Commit:** `feat(cache-core): add ready_content_hashes for capability advertisement`

---

### Task 3: Wire delivery capabilities into CapacityAnnouncement

**Files:**
- Modify: `steward/node/src/pod/capacity.rs`

**What to build:**

The CapacityAnnouncement already has `capabilities: Vec<String>`. When constructing the announcement, append delivery capability strings from `NodeCapabilities::delivery.to_capability_strings()`.

This is additive — existing capability strings (e.g., `"path-recommendation"`) remain, delivery strings are appended.

The ExtractionCache's `ready_content_hashes()` should be called to populate `ready_content` before generating the announcement. This means the announcement builder needs access to the extraction cache (or receives the ready_content list as a parameter).

**Tests:**
- Unit test: announcement with delivery capabilities includes expected strings
- Unit test: `"warm:{hash}"` entries appear when ready_content is non-empty

**Verify:** `cd steward/node && RUSTFLAGS="" cargo check`

**Commit:** `feat(steward): include delivery capabilities in CapacityAnnouncement`

---

### Task 4: HEAD /_capability endpoint on elohim-storage

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

**What to build:**

Add a route handler for `HEAD /apps/{app_id}/_capability`. This is a lightweight probe — no body, just headers:

```
HEAD /apps/evolution-of-trust/_capability

Response headers:
  X-Delivery-Mode: extracted | compressed | blob-only
  X-Blob-Hash: sha256-abc123...
  X-Cache-Tier: projection | extraction | blob-only
  X-Ready: true | false  (whether this specific app_id is warm)
  Content-Length: 0
```

Logic:
1. Parse app_id from path
2. Look up blob_hash from app_index
3. Check if extraction cache has it warm
4. Return capability headers

Add the route match in the main dispatch:
```rust
(Method::HEAD, p) if p.starts_with("/apps/") && p.ends_with("/_capability") => {
    // extract app_id, return capability headers
}
```

**Tests:**
- Unit test: returns correct headers when cache is warm
- Unit test: returns `X-Ready: false` when cache is cold
- Unit test: returns 404 when app_id is unknown

**Verify:** `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Commit:** `feat(storage): HEAD /_capability endpoint for delivery negotiation`

---

### Task 5: HEAD /_capability endpoint on doorway

**Files:**
- Modify: `doorway/doorway-service/src/routes/apps.rs`
- Modify: `doorway/doorway-service/src/server/http.rs` (route dispatch)

**What to build:**

Add a `HEAD /apps/{app_id}/_capability` handler to doorway. Doorway reports its own capability (projection cache tier):

```
HEAD /apps/evolution-of-trust/_capability

Response headers:
  X-Delivery-Mode: extracted (doorway always serves extracted from MongoDB)
  X-Blob-Hash: sha256-abc123...
  X-Cache-Tier: projection
  X-Ready: true | false  (whether app_id is in MongoDB cache)
  Content-Length: 0
```

Logic:
1. If `state.app_file_cache` is Some: check if blob_hash is known for app_id
2. Report `X-Cache-Tier: projection`, `X-Ready: true/false`
3. If no cache: fall through to storage proxy for its capability response

Add dispatch in http.rs:
```rust
(Method::HEAD, p) if p.starts_with("/apps/") && p.ends_with("/_capability") => {
    // doorway capability response
}
```

**Tests:**
- Unit test: returns projection tier headers when cache service exists
- Unit test: proxies to storage when no cache service

**Verify:** `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins`

**Commit:** `feat(doorway): HEAD /_capability endpoint for delivery negotiation`

---

## Part B: Service Worker

### Task 6: Create raw Service Worker with /apps/ intercept

**Files:**
- Create: `app/elohim-app/src/apps-sw.ts`
- Modify: `app/elohim-app/src/main.ts` (registration)
- Modify: `app/elohim-app/angular.json` (include SW in assets)

**What to build:**

A raw Service Worker (not @angular/service-worker) that:
1. Intercepts `fetch` events for URLs matching `/apps/`
2. Checks CacheStorage first (cache key: `apps-v1`)
3. On hit: return cached Response
4. On miss: fetch from network, cache response, return
5. Exposes a `message` handler for invalidation commands from the Angular app

Registration in `main.ts`:
```typescript
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/apps-sw.js', { scope: '/apps/' })
      .then(reg => console.log('Apps SW registered:', reg.scope))
      .catch(err => console.warn('Apps SW registration failed:', err));
  });
}
```

The SW file needs to be compiled from TypeScript and included in the build output. Add to `angular.json` assets array or configure a separate build step.

**Important:** The SW scope is `/apps/` — it only intercepts requests to the app serving route, not the entire Angular app. This is narrowly scoped by design.

**Tests:**
- Manual verification: SW registers and shows in DevTools > Application > Service Workers
- Manual verification: Network tab shows "(ServiceWorker)" source for /apps/ requests after first load

**Commit:** `feat(app): register raw Service Worker for /apps/ intercept`

---

### Task 7: SW capability probe and delivery mode selection

**Files:**
- Modify: `app/elohim-app/src/apps-sw.ts`

**What to build:**

Before fetching app files, the SW sends a `HEAD /_capability` probe to determine the delivery strategy:

```typescript
async function probeCapability(appId: string): Promise<DeliveryInfo> {
  const resp = await fetch(`/apps/${appId}/_capability`, { method: 'HEAD' });
  return {
    deliveryMode: resp.headers.get('X-Delivery-Mode') || 'unknown',
    blobHash: resp.headers.get('X-Blob-Hash') || '',
    cacheTier: resp.headers.get('X-Cache-Tier') || 'unknown',
    ready: resp.headers.get('X-Ready') === 'true',
  };
}
```

Cache the probe result for the duration of the app load (don't re-probe for every file). Invalidate when a new blob_hash is detected.

The probe determines the fetch strategy:
- `deliveryMode === 'extracted'` → fetch individual files (Task 8)
- `deliveryMode === 'compressed'` or `deliveryMode === 'blob-only'` → fetch ZIP, extract locally (Task 9)

**Commit:** `feat(app): SW capability probe for delivery mode selection`

---

### Task 8: SW cache-first fetch for extracted delivery

**Files:**
- Modify: `app/elohim-app/src/apps-sw.ts`

**What to build:**

When the peer serves extracted files, the SW does cache-first fetch:

```typescript
self.addEventListener('fetch', (event: FetchEvent) => {
  const url = new URL(event.request.url);
  if (!url.pathname.startsWith('/apps/')) return;
  if (url.pathname.endsWith('/_capability')) return; // don't cache probes

  event.respondWith(handleAppFetch(event.request));
});

async function handleAppFetch(request: Request): Promise<Response> {
  // 1. Check CacheStorage
  const cache = await caches.open('apps-v1');
  const cached = await cache.match(request);
  if (cached) return cached;

  // 2. Fetch from network
  const response = await fetch(request);
  if (response.ok) {
    cache.put(request, response.clone());
  }
  return response;
}
```

The cache key includes the URL (which already includes app_id and file_path). The blob_hash-based invalidation (Task 10) handles staleness.

**Commit:** `feat(app): SW cache-first fetch for extracted app files`

---

### Task 9: SW ZIP extraction for compressed delivery

**Files:**
- Modify: `app/elohim-app/src/apps-sw.ts`
- Modify: `app/elohim-app/package.json` (add `jszip` dependency)

**What to build:**

When the peer only serves compressed (ZIP), the SW downloads the entire ZIP once and extracts all files into CacheStorage:

```typescript
import JSZip from 'jszip';

async function fetchAndExtractZip(appId: string, blobHash: string): Promise<void> {
  const zipResp = await fetch(`/blob/${blobHash}`);
  const zipData = await zipResp.arrayBuffer();
  const zip = await JSZip.loadAsync(zipData);

  const cache = await caches.open('apps-v1');
  const entries = Object.entries(zip.files);

  for (const [path, file] of entries) {
    if (file.dir) continue;
    const data = await file.async('arraybuffer');
    const contentType = guessContentType(path);
    const response = new Response(data, {
      headers: { 'Content-Type': contentType }
    });
    await cache.put(new Request(`/apps/${appId}/${path}`), response);
  }
}
```

The SW tries individual fetch first. If the capability probe says `compressed`, it switches to ZIP extraction. After extraction, all subsequent requests are served from CacheStorage.

**Important:** JSZip works in Service Worker context (no DOM dependency). ~45KB gzipped.

**Commit:** `feat(app): SW ZIP extraction for compressed-only delivery`

---

### Task 10: SW cache invalidation via BroadcastChannel

**Files:**
- Modify: `app/elohim-app/src/apps-sw.ts`
- Create: `app/elohim-app/src/app/elohim/services/sw-bridge.service.ts`

**What to build:**

The Angular app notifies the SW when content is re-seeded (new blob_hash):

**In the SW:**
```typescript
const channel = new BroadcastChannel('apps-sw');
channel.onmessage = async (event) => {
  if (event.data.type === 'invalidate') {
    const { appId } = event.data;
    const cache = await caches.open('apps-v1');
    const keys = await cache.keys();
    const toDelete = keys.filter(req =>
      new URL(req.url).pathname.startsWith(`/apps/${appId}/`)
    );
    await Promise.all(toDelete.map(key => cache.delete(key)));
  }
};
```

**In Angular (`sw-bridge.service.ts`):**
```typescript
@Injectable({ providedIn: 'root' })
export class SwBridgeService {
  private channel = typeof BroadcastChannel !== 'undefined'
    ? new BroadcastChannel('apps-sw')
    : null;

  invalidateApp(appId: string): void {
    this.channel?.postMessage({ type: 'invalidate', appId });
  }
}
```

Wire the `SwBridgeService` to fire when content update signals arrive (e.g., from the existing signal subscription infrastructure).

**Commit:** `feat(app): SW cache invalidation via BroadcastChannel`

---

### Task 11: Build pipeline integration + Tauri verification

**Files:**
- Modify: `app/elohim-app/angular.json` (verify SW included in build)
- Modify: `Jenkinsfile` (if build step needed for SW TypeScript compilation)

**What to build:**

1. Verify the SW TypeScript compiles and is included in the production build output
2. Verify the SW registers correctly in Tauri WebView (may need CSP or scope adjustment)
3. Verify offline: load an HTML5 app, disconnect network, reload — app still works

**Verify:**
```bash
cd app/elohim-app && pnpm run build  # Production build
ls dist/elohim-app/browser/apps-sw.js  # SW file exists in output
```

**Commit:** `feat(app): verify SW in production build and Tauri WebView`

---

## Verification Checklist

After all tasks, run:

```bash
# Rust - storage
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test

# Rust - doorway
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins

# Rust - steward
cd steward/node && RUSTFLAGS="" cargo check

# Rust - cache-core
cd elohim/elohim-cache-core && cargo test

# Angular - build + test
cd app/elohim-app && pnpm run build && pnpm test
```

## Key Files Reference

| File | Purpose |
|------|--------|
| `elohim/elohim-storage/src/identity.rs` | DeliveryCapabilities + CacheTier (Task 1) |
| `elohim/elohim-cache-core/src/extraction/cache.rs` | ready_content_hashes (Task 2) |
| `steward/node/src/pod/capacity.rs` | CapacityAnnouncement with delivery strings (Task 3) |
| `elohim/elohim-storage/src/http.rs` | HEAD /_capability on storage (Task 4) |
| `doorway/doorway-service/src/routes/apps.rs` | HEAD /_capability on doorway (Task 5) |
| `app/elohim-app/src/apps-sw.ts` | Service Worker (Tasks 6-10) |
| `app/elohim-app/src/main.ts` | SW registration (Task 6) |
| `app/elohim-app/src/app/elohim/services/sw-bridge.service.ts` | Angular↔SW bridge (Task 10) |
