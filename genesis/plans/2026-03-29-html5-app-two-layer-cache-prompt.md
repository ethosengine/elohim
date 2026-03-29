# HTML5 App Two-Layer Cache — Plan & Execute Prompt

## Problem

HTML5 apps (like Evolution of Trust) are served via `/apps/{app_id}/{file_path}`. Every single file request (JS, CSS, images — 30+ concurrent on page load) triggers:

1. **SQLite query** — grab pool connection, scan all `html5-app` content to find matching `appId`
2. **Read 6.75MB ZIP** from blob store on disk
3. **Decompress ZIP in memory**
4. **Extract single file** from the archive
5. Return bytes

This causes two failures:
- **SQLite pool exhaustion** — 10-connection pool (r2d2) exhausted by 30+ concurrent requests → `timed out waiting for connection` → HTTP 500
- **Memory pressure** — 30 × 6.75MB concurrent decompression in a 256Mi container → near OOM

## Root Cause Files

- **App serving handler**: `elohim/elohim-storage/src/http.rs:2639-2778` (`handle_app_request`)
  - Line 2677: `self.get_conn()` — grabs SQLite pool connection per request
  - Line 2686: `list_content()` — scans ALL html5-app content (up to 100) to find matching appId
  - Line 2767: `self.blob_store.get(&blob_hash)` — reads full ZIP per request
  - ZIP is decompressed in memory via `zip::ZipArchive` every time
- **Pool config**: `elohim/elohim-storage/src/db/mod.rs:99-107` — hardcoded 10 connections, 30s timeout
- **Container limit**: `genesis/orchestrator/manifests/edgenode/alpha.yaml:277` — 256Mi for elohim-storage

## Architecture: Two-Layer Caching

Two distinct serving paths need their own caching strategy:

### Layer 1: Storage Disk Extraction (peer-to-peer path)

When storage serves app files directly (Tauri desktop, peer requests), it should extract the ZIP once to disk and serve static files thereafter.

**Design constraints:**
- Storage container has a PVC at `/data` (persistent across restarts)
- Extract to `/data/apps/{app_id}/` on first request
- Serve directly from disk on subsequent requests — no SQLite query, no ZIP read, no decompression
- Invalidate when blob hash changes (check hash before serving cached version)
- Storage memory limit is 256Mi — disk extraction avoids memory pressure entirely

**Suggested approach:**
1. On first `/apps/{app_id}/{file}` request:
   - Check if `/data/apps/{app_id}/.blob_hash` exists and matches current blob hash
   - If cache hit: serve file directly from `/data/apps/{app_id}/{file}` (no DB, no ZIP)
   - If cache miss: extract full ZIP to `/data/apps/{app_id}/`, write `.blob_hash` marker, then serve
2. The SQLite query to find the content record + blob hash can be cached in a `HashMap<String, String>` (appId → blobHash) refreshed on startup or TTL — avoids the per-request content scan
3. No pool connection needed for cache-hit path

### Layer 2: Doorway Projection Cache (web2 path)

When doorway proxies `/apps/` requests from browsers, it should cache extracted files in its MongoDB projection cache so storage isn't hit at all for repeat requests.

**Design constraints:**
- Doorway already has MongoDB projection cache infrastructure (`alpha-mongodb.yaml`)
- Doorway proxies `/apps/` to storage today — it's a pass-through
- Cache key: `apps:{app_id}:{file_path}` with blob hash as version
- Cache should store the extracted file bytes + content-type
- TTL: long (hours/days) since app content rarely changes. Invalidate on blob hash change.
- This layer handles the browser's 30+ concurrent requests — most should be cache hits after the first load

**Suggested approach:**
1. Doorway intercepts `/apps/{app_id}/{file_path}` before proxying
2. Check MongoDB cache for `apps:{app_id}:{file_path}`
3. If hit: return cached bytes + content-type directly (zero storage load)
4. If miss: proxy to storage, cache the response bytes in MongoDB, return to client
5. Invalidation: when content is seeded/updated, doorway clears the `apps:{app_id}:*` cache entries

## Priority

**Layer 1 (storage disk extraction) is the critical fix.** It solves the pool exhaustion and OOM risk regardless of whether requests come through doorway or directly. Layer 2 is an optimization that reduces load on storage when serving many browser clients.

## Existing Code Context

- **Blob store**: `elohim/elohim-storage/src/blob_store.rs` — handles blob read/write to disk
- **HTTP handler**: `elohim/elohim-storage/src/http.rs` — all routes including `/apps/`
- **Doorway proxy**: `doorway/doorway-service/src/routes/` — HTTP routes that proxy to storage
- **Doorway cache**: search for `projection`, `cache`, `mongodb` in doorway-service
- **K8s manifests**: `genesis/orchestrator/manifests/edgenode/alpha.yaml` (storage container), `genesis/orchestrator/manifests/doorway/alpha.yaml` (doorway), `genesis/orchestrator/manifests/infra/alpha-mongodb.yaml` (MongoDB)

## Current Content

Only 1 HTML5 app exists today: `simulation-evolution-of-trust` (6.75MB ZIP, ~30 files). More may come. The ZIP was uploaded by the seeder (`genesis/seeder/src/seed.ts` Phase 0 blob upload).

## Acceptance Criteria

1. Loading Evolution of Trust on alpha produces zero HTTP 500 errors
2. Storage container stays well under 256Mi during concurrent app file requests
3. After first load, subsequent requests serve from cache (disk or MongoDB) without ZIP decompression
4. `kubectl logs -c elohim-storage` shows no `timed out waiting for connection` errors during app load
5. Cache invalidates correctly when app content is re-seeded with a new blob hash

## Instructions

Use `/superpowers:writing-plans` to create an implementation plan from this prompt. Execute Layer 1 first (storage disk extraction), verify on alpha, then Layer 2 (doorway projection cache) as a follow-up.
