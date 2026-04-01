# Content Addressing Cleanup — Design Spec

> Sprint: 2026-03-31 | Approach: Compiler-driven rename (IoC)

## Problem

`appId` means three different things depending on context:

| Context | Current field | Value example | Meaning |
|---------|--------------|---------------|---------|
| DB column / API response | `app_id` / `appId` | `"lamad"` | Holochain app context (multi-tenant scope) |
| Inside `contentBody` JSON | `appId` | `"evolution-of-trust"` | HTML5 app slug (URL path) |
| Holochain conductor | `installed_app_id` | `"lamad"` | Conductor-level installed app |

This caused a production bug where doorway's projection cache read the Holochain context instead of the content slug when resolving HTML5 app files.

Additionally:
- App URLs use slugs (`/apps/{slug}/file`) with no content-addressed alternative
- ContentProjection uses snake_case field lookups against camelCase data (silent extraction failures)

## Decisions

1. **Rename Holochain app context**: `app_id` → `h_app_id` everywhere (DB column, Rust structs, API boundary). Serde `rename_all = "camelCase"` produces `hAppId`.
2. **Rename HTML5 app identifier**: `contentBody.appId` → `contentBody.slug`. Content declares its own URL slug (WordPress model).
3. **CID URL support**: Both `/apps/{slug}/file` and `/apps/{blob_hash}/file` serve content. `X-Content-Address` header on all responses. SW caches by blob_hash.
4. **Fix ContentProjection**: Change snake_case lookups to camelCase to match actual wire format.
5. **DB wipe + reseed**: No migrations. Rename columns directly in `schema.rs`.

## Three distinct identifiers after cleanup

| Name | Rust field | JSON key | Purpose |
|------|-----------|----------|---------|
| `h_app_id` | `h_app_id` | `hAppId` | Holochain app context / multi-tenant scope |
| `slug` | (inside contentBody) | `slug` | Human-readable URL path, declared by content |
| `blob_hash` | `blob_hash` | `blobHash` | Content address (CID), cache key, immutable |

## Execution Strategy

Change the source of truth → compile → fix every error the compiler reports → test. No grep-and-hope.

## Phase 1: Naming Consistency

### Step 1a — Rust schema + models (elohim-storage)

**Source of truth**: `elohim/elohim-storage/src/schema.rs`
- Rename column `app_id` → `h_app_id` in every table definition

**Models**: `elohim/elohim-storage/src/db/models.rs`
- Rename field `app_id` → `h_app_id` in all ~42 Diesel model structs
- No `#[diesel(column_name)]` needed — column and field match after schema.rs change

**Verification**: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check` — fix every error.

### Step 1b — Rust API boundary (elohim-storage)

**Views**: `elohim/elohim-storage/src/views.rs`
- Rename `app_id` → `h_app_id` in all 8 View structs (ContentView, RelationshipView, HumanRelationshipView, ContributorPresenceView, EconomicEventView, ContentMasteryView, StewardshipAllocationView, HumanView)
- `#[serde(rename_all = "camelCase")]` automatically produces `hAppId`

**HTTP handlers**: `elohim/elohim-storage/src/http.rs`
- All `app_id` references in route handlers, `AppContext`, JSON responses → `h_app_id`
- `load_app_index()` and `lookup_app_blob_hash()` — see Step 1d for slug rename

**Verification**: `cargo check` — fix From impls, handler params, JSON construction.

### Step 1c — Generated TypeScript types

**Action**: `cargo test export_bindings` regenerates `elohim/sdk/storage-client-ts/src/generated/`
- Generated types will have `h_app_id` (ts-rs preserves Rust field names)
- Note: ts-rs does NOT apply serde rename_all. The generated types use Rust field names. Consumers that read from the HTTP API use `hAppId` (camelCase). Consumers that use generated types use `h_app_id`. This matches the existing pattern where generated types have `app_id` (snake_case).

**Verification**: `tsc --noEmit` on storage-client-ts and downstream packages.

### Step 1d — Content body slug rename

**Rust (elohim-storage)**:
- `http.rs` `load_app_index()` (~line 280): `obj.get("appId")` → `obj.get("slug")`
- `http.rs` `lookup_app_blob_hash()` (~line 3088): same change
- Rename `app_index` field → `slug_index` (type stays `HashMap<String, String>`, semantics: slug → blob_hash)

**Rust (doorway-service)**:
- `cache/app_file_cache.rs`: both `extract_html5_app_id()` functions → `extract_html5_slug()`
  - `data.get("contentBody")` → parse JSON → `body.get("slug")` instead of `body.get("appId")`
- `app_index` field → `slug_index`
- Resolve the existing TODO at line 410

**TypeScript (Angular)**:
- `iframe-renderer.component.ts`: `Html5AppContent.appId` → `Html5AppContent.slug`
- `html5-app-format.plugin.ts`: `appId` field in content object → `slug`, validation regex unchanged
- `apps-sw.ts`: variable names `appId` → `slug` (URL parsing, cache keys, delivery cache)

**Seeder**:
- `genesis/seeder/src/doorway-client.ts`: `registerApp()` sends `slug` instead of `appId`
- `genesis/seeder/src/seed.ts`: content object construction uses `slug`
- Any seed JSON fixtures with `appId` in content body → `slug`

**Verification**: `cargo check` on both crates, `tsc --noEmit` on Angular packages.

### Step 1e — ContentProjection camelCase fix

**File**: `doorway/doorway-service/src/projection/collections/content.rs`

In `from_entry()` (~lines 151-186), change lookups:
- `data.get("content_format")` → `data.get("contentFormat")`
- `data.get("trust_score")` → `data.get("trustScore")`
- `data.get("estimated_minutes")` → `data.get("estimatedMinutes")`
- `data.get("content")` → `data.get("content")` (unchanged, single word)

Audit for any other snake_case lookups against the `data` JsonValue in the projection layer.

**Verification**: `cargo check` + `cargo test` on doorway-service.

### Step 1f — Wipe and verify

1. `cargo test` on elohim-storage
2. `cargo test` on doorway-service
3. Wipe SQLite DB
4. Reseed
5. Verify projected documents in MongoDB have `hAppId` field
6. Verify HTML5 app content body has `slug` field

## Phase 2: CID URL Support

### Step 2a — Storage dual-path resolution

**File**: `elohim/elohim-storage/src/http.rs`

In `handle_app_request()`:
- After extracting the first path segment (currently treated as slug), check if it matches `sha256-{hex}` pattern
- If yes: use directly as `blob_hash` (skip slug_index lookup)
- If no: treat as slug, resolve via `slug_index` as before
- Add `X-Content-Address: {blob_hash}` header to all `/apps/` responses

```rust
let is_content_address = identifier.starts_with("sha256-");
let blob_hash = if is_content_address {
    identifier.to_string()
} else {
    self.resolve_slug(identifier).await?
};
```

### Step 2b — Doorway dual-path resolution

**File**: `doorway/doorway-service/src/routes/apps.rs`

- `parse_app_path()` returns `(identifier, file_path)` — identifier is either slug or blob_hash
- `resolve_blob_hash()` short-circuits if identifier already IS a blob_hash
- Forward `X-Content-Address` header from storage response through to client

**File**: `doorway/doorway-service/src/cache/app_file_cache.rs`
- `resolve_blob_hash()` returns the identifier directly if it matches blob_hash pattern
- Cache keys already use blob_hash — no change needed for cache storage

### Step 2c — Service Worker CID caching

**File**: `app/elohim-app/src/apps-sw.ts`

- Read `X-Content-Address` header from fetch response
- Cache entry key: `/apps/{blob_hash}/{file_path}` (not the request URL)
- On fetch with slug URL: resolve to blob_hash (from delivery cache or capability probe), check cache under CID key first
- `probeCapability()` already returns `blobHash` — use it for cache key construction
- ZIP extraction: cache entries under `/apps/{blob_hash}/{path}` instead of `/apps/{slug}/{path}`
- Invalidation: BroadcastChannel message includes both `slug` and `blobHash`, clear both key patterns

### Step 2d — Response headers

All `/apps/` responses include:
- `X-Content-Address: {blob_hash}` — the CID for this content
- `X-Content-Slug: {slug}` — the human-readable identifier (if resolved from slug)
- Existing headers unchanged: `Cross-Origin-Resource-Policy`, `Cross-Origin-Embedder-Policy`, `X-Cache`

### Step 2e — Angular iframe-renderer

**No change needed.** The renderer constructs `/apps/{slug}/{entryPoint}` — human-readable URLs. The SW transparently caches by CID. The doorway serves both URL forms.

### Step 2f — Verify

1. `GET /apps/evolution-of-trust/index.html` serves content, returns `X-Content-Address` header
2. `GET /apps/sha256-{hash}/index.html` serves same content
3. SW caches under CID key — second request to either URL is a cache hit
4. Re-seed with updated content (new blob_hash) → old slug URL serves new content, old CID URL 404s (correct — immutable)

## A2O Regression Scenarios

Add to `genesis/a2o/features/delivery/`:

```gherkin
Scenario: Content with slug URL resolves to CID-based cache entry
  Given an HTML5 app "evolution-of-trust" is seeded with blob hash "sha256-abc123"
  When a learner navigates to "/apps/evolution-of-trust/index.html"
  Then the response includes header "X-Content-Address: sha256-abc123"
  And the service worker caches the response under "/apps/sha256-abc123/index.html"
  When the learner navigates to "/apps/sha256-abc123/index.html"
  Then the response is served from cache

Scenario: Re-seeded content with new CID invalidates old slug mapping
  Given an HTML5 app "evolution-of-trust" was seeded with blob hash "sha256-old"
  And the service worker has cached files under "/apps/sha256-old/"
  When the app is re-seeded with blob hash "sha256-new"
  Then navigating to "/apps/evolution-of-trust/index.html" serves the new content
  And the response includes header "X-Content-Address: sha256-new"
  And the old cache entries under "/apps/sha256-old/" are invalidated
```

## Files Touched (complete list)

### Phase 1
| File | Change |
|------|--------|
| `elohim/elohim-storage/src/schema.rs` | `app_id` → `h_app_id` in all table macros |
| `elohim/elohim-storage/src/db/models.rs` | `app_id` → `h_app_id` in ~42 structs |
| `elohim/elohim-storage/src/views.rs` | `app_id` → `h_app_id` in 8 View structs |
| `elohim/elohim-storage/src/http.rs` | `app_id` → `h_app_id` in handlers; `app_index` → `slug_index`; content body reads `slug` |
| `elohim/elohim-storage/src/db/*.rs` | Fix any query helpers referencing `app_id` |
| `doorway/doorway-service/src/cache/app_file_cache.rs` | `app_index` → `slug_index`; extract functions read `slug`; resolve TODO |
| `doorway/doorway-service/src/projection/collections/content.rs` | Fix snake_case → camelCase lookups |
| `doorway/doorway-service/src/projection/document.rs` | Any `app_id` field references → `h_app_id` |
| `doorway/doorway-service/src/routes/apps.rs` | Variable names: `app_id` → `slug` where it's the HTML5 identifier |
| `doorway/doorway-service/src/routes/auth_routes.rs` | `app_id` → `h_app_id` where it's Holochain context |
| `doorway/doorway-service/src/routes/admin_conductors.rs` | `app_id` → `h_app_id` |
| `doorway/doorway-service/src/db/schemas/app_file_cache.rs` | MongoDB field `app_id` stays (it's the slug in cache context — rename to `slug`) |
| `elohim/sdk/storage-client-ts/src/generated/*.ts` | Regenerated via `cargo test export_bindings` |
| `elohim/sdk/storage-client-ts/src/types.ts` | `appId` → `hAppId` in StorageConfig |
| `elohim/sdk/storage-client-ts/src/client.ts` | URL path construction uses `hAppId` |
| `app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts` | `Html5AppContent.appId` → `.slug` |
| `app/elohim-app/src/app/lamad/content-io/plugins/html5-app/html5-app-format.plugin.ts` | `appId` → `slug` in content construction + validation |
| `app/elohim-app/src/apps-sw.ts` | Variable names `appId` → `slug` |
| `app/elohim-app/src/app/elohim/adapters/*.ts` | Any `appId` field mapping → `hAppId` |
| `app/elohim-library/projects/elohim-service/src/models/holochain.model.ts` | `appId` → `hAppId` in config interfaces |
| `app/elohim-library/projects/elohim-service/src/client/types.ts` | `appId` → `hAppId` in HolochainConnection |
| `elohim/sdk/src/connection.ts` | `appId` → `hAppId` in ConnectionState, ConnectionConfig |
| `elohim/sdk/src/types.ts` | `DEFAULT_APP_ID` stays, field name `appId` → `hAppId` |
| `elohim/holochain/rna/typescript/src/config.ts` | `appId` → `hAppId` |
| `app/elohim-app/src/environments/environment.types.ts` | `holochainAppId` → `holochainHAppId` |
| `app/elohim-app/src/app.config.ts` | Field mapping update |
| `genesis/seeder/src/doorway-client.ts` | `registerApp()` sends `slug` |
| `genesis/seeder/src/seed.ts` | Content construction uses `slug`; `APP_ID` constant context clarified |

### Phase 2
| File | Change |
|------|--------|
| `elohim/elohim-storage/src/http.rs` | Dual-path resolution (slug or blob_hash), `X-Content-Address` header |
| `doorway/doorway-service/src/routes/apps.rs` | Dual-path resolution, header forwarding |
| `doorway/doorway-service/src/cache/app_file_cache.rs` | Short-circuit for blob_hash identifiers |
| `app/elohim-app/src/apps-sw.ts` | Cache by CID key, read `X-Content-Address` header |
| `genesis/a2o/features/delivery/*.feature` | Regression scenarios |

## Out of Scope

- Holochain conductor's `installed_app_id` — this is Holochain's naming, not ours
- CIDv1 (`bafkrei...`) format migration — blob_hash stays `sha256-{hex}` for now
- EPR URI resolution changes
- Database migration scripts (wipe + reseed instead)
