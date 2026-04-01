# Content Addressing Cleanup — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Disambiguate three app identifiers (`hAppId`, `slug`, `blobHash`), add CID-based URL support, fix ContentProjection camelCase bug.

**Architecture:** Compiler-driven rename — change the source of truth (schema/struct), compile, fix every error. Two phases: Phase 1 (naming consistency, pure rename), Phase 2 (CID URL support, new behavior). DB wipe + reseed after Phase 1.

**Tech Stack:** Rust (Diesel ORM, serde, hyper), TypeScript (Angular 19, Service Worker), MongoDB (doorway projection)

**Design spec:** `genesis/plans/2026-03-31-content-addressing-cleanup-design.md`

---

## Phase 1: Naming Consistency

### Task 1: Rename `app_id` → `h_app_id` in elohim-storage (Rust source of truth)

This is the big mechanical rename. Change schema, models, context, views, and let `cargo check` cascade errors through http.rs and db modules. Fix every error.

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (25 tables)
- Modify: `elohim/elohim-storage/src/db/models.rs` (~42 structs)
- Modify: `elohim/elohim-storage/src/db/context.rs`
- Modify: `elohim/elohim-storage/src/views.rs` (8 View structs)
- Modify: `elohim/elohim-storage/src/http.rs` (handlers, JSON responses)
- Modify: `elohim/elohim-storage/src/db/*_diesel.rs` (query helpers)

- [ ] **Step 1: Rename column in diesel_schema.rs**

Find-and-replace `app_id` → `h_app_id` in all `table!` macro invocations. There are 25 tables with this column:

`comments`, `collectives`, `collective_participations`, `content`, `content_mastery`, `content_tags`, `contributor_presences`, `enum_registry`, `economic_events`, `human_relationships`, `relationships`, `schedules`, `places`, `spatial_contexts`, `stewardship_allocations`, `custodian_metrics`, `humans`, `steward_affinity`, `rea_commitments`, `agreements`, `stewarded_nodes`, `knowledge_maps`, `imagodei_observations`, `hazards`, `risk_alerts`

Use your editor's replace-all within this file. Every instance of `app_id -> Text,` becomes `h_app_id -> Text,`.

- [ ] **Step 2: Rename field in models.rs**

Find-and-replace `app_id` → `h_app_id` in all struct field declarations. There are ~42 structs (readback + insert pairs for each table). The pattern is:

```rust
// Before:
pub app_id: String,      // readback models
pub app_id: &'a str,     // insert models (NewX<'a>)

// After:
pub h_app_id: String,
pub h_app_id: &'a str,
```

Use replace-all within the file. This is safe because `app_id` is only used as a struct field name in this file.

- [ ] **Step 3: Rename in context.rs**

```rust
// File: elohim/elohim-storage/src/db/context.rs

// Before (line 10):
pub app_id: String,

// After:
pub h_app_id: String,
```

Also rename all method references:

```rust
// Before (lines 15-18):
pub fn new(app_id: impl Into<String>) -> Self {
    Self {
        app_id: app_id.into(),
    }
}

// After:
pub fn new(h_app_id: impl Into<String>) -> Self {
    Self {
        h_app_id: h_app_id.into(),
    }
}

// Before (lines 31-33):
pub fn app_id(&self) -> &str {
    &self.app_id
}

// After:
pub fn h_app_id(&self) -> &str {
    &self.h_app_id
}

// Before (line 46):
write!(f, "AppContext({})", self.app_id)

// After:
write!(f, "AppContext({})", self.h_app_id)
```

Update tests at bottom of file:

```rust
// Before:
assert_eq!(AppContext::default_lamad().app_id, "lamad");
assert_eq!(AppContext::default_elohim().app_id, "elohim");
assert_eq!(AppContext::default().app_id, "lamad");
let ctx = AppContext::new("calendar");
assert_eq!(ctx.app_id, "calendar");

// After:
assert_eq!(AppContext::default_lamad().h_app_id, "lamad");
assert_eq!(AppContext::default_elohim().h_app_id, "elohim");
assert_eq!(AppContext::default().h_app_id, "lamad");
let ctx = AppContext::new("calendar");
assert_eq!(ctx.h_app_id, "calendar");
```

- [ ] **Step 4: Rename in views.rs**

Find-and-replace `app_id` → `h_app_id` in all 8 View structs: `ContentView`, `RelationshipView`, `HumanRelationshipView`, `ContributorPresenceView`, `EconomicEventView`, `ContentMasteryView`, `StewardshipAllocationView`, `HumanView`.

All structs have `#[serde(rename_all = "camelCase")]` so `h_app_id` automatically serializes as `hAppId` in JSON.

Also update all `From<Model> for View` impl blocks that map `app_id`:

```rust
// Before (in each From impl):
app_id: c.app_id,

// After:
h_app_id: c.h_app_id,
```

- [ ] **Step 5: Compile and fix cascading errors**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | head -100
```

The compiler will report every remaining reference to `app_id` that needs updating. These will be in:
- `http.rs`: route handlers that construct `AppContext`, JSON `"appId"` keys in sync responses, path extraction for `/db/{app_id}/...` routes
- `db/*_diesel.rs`: query builders that filter on `app_id` column

For each error, rename `app_id` → `h_app_id` in the Rust code. For JSON response keys in sync handlers (lines ~1388, 1438, 1486, 1549, 1609), rename:

```rust
// Before:
"appId": app_id,

// After:
"hAppId": h_app_id,
```

**Important**: The local variable names in http.rs route handlers (e.g., `let app_id = ...` from URL path extraction) should ALSO be renamed to `h_app_id` for consistency, since they represent the Holochain app context.

Repeat `cargo check` until zero errors.

- [ ] **Step 6: Run tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -30
```

Expected: All tests pass. If any fail, fix and re-run.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "$(cat <<'EOF'
refactor(storage): rename app_id → h_app_id for Holochain app context

Disambiguates the Holochain app context (hAppId, e.g. "lamad") from
HTML5 app slug and blob hash. Schema, models, views, handlers all
renamed. JSON API responses now use "hAppId" instead of "appId".

Part of content addressing cleanup sprint.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Rename `app_id` → `h_app_id` in doorway-service

The doorway has `app_id` in conductor admin routes and the app file cache schema. These represent the Holochain app context and need renaming.

**Files:**
- Modify: `doorway/doorway-service/src/routes/admin_conductors.rs`
- Modify: `doorway/doorway-service/src/db/schemas/app_file_cache.rs` (MongoDB `app_id` field → `slug` — see note)

**Note on app_file_cache.rs**: The `app_id` field in `AppFileCacheDoc` is actually the HTML5 app slug (used for cache key construction like `"{app_id}:{file_path}:{blob_hash}"`). This should be renamed to `slug`, not `h_app_id`. This is covered in Task 4.

- [ ] **Step 1: Rename in admin_conductors.rs**

Three structs have `app_id` fields representing the Holochain app context:

```rust
// AgentSummary (line 52): app_id → h_app_id
pub struct AgentSummary {
    pub agent_pub_key: String,
    pub h_app_id: String,       // was: app_id
    pub assigned_at: String,
}

// AgentConductorResponse (line 72): app_id → h_app_id
pub struct AgentConductorResponse {
    pub agent_pub_key: String,
    pub conductor_id: String,
    pub conductor_url: String,
    pub h_app_id: String,       // was: app_id
    pub assigned_at: String,
}

// AssignAgentRequest (line 193): app_id → h_app_id
pub struct AssignAgentRequest {
    pub agent_pub_key: String,
    pub conductor_id: String,
    #[serde(default = "default_h_app_id")]
    pub h_app_id: String,       // was: app_id
}

// Rename the default function (line 196):
fn default_h_app_id() -> String {
    "elohim".to_string()
}
```

All three structs have `#[serde(rename_all = "camelCase")]`, so `h_app_id` → `hAppId` in JSON.

Then fix all references in handler functions that use these fields (e.g., `entry.app_id` → `entry.h_app_id`, `request.app_id` → `request.h_app_id`). Use `cargo check` to find them all:

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | head -100
```

**Note**: `auth_routes.rs` uses `installed_app_id` — this is Holochain's own naming. Do NOT rename it.

- [ ] **Step 2: Run tests**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -30
```

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/routes/admin_conductors.rs
git commit -m "$(cat <<'EOF'
refactor(doorway): rename app_id → h_app_id in conductor admin routes

Consistent with storage rename. AgentSummary, AgentConductorResponse,
and AssignAgentRequest now use hAppId in JSON responses.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Fix ContentProjection camelCase bug

The `from_entry()` method uses snake_case field lookups (`content_format`, `trust_score`, `estimated_minutes`) but the `data` field contains camelCase from elohim-storage's serde output. This causes silent extraction failures.

**Files:**
- Modify: `doorway/doorway-service/src/projection/collections/content.rs`

- [ ] **Step 1: Fix field lookups in from_entry()**

```rust
// File: doorway/doorway-service/src/projection/collections/content.rs
// In from_entry() method (~lines 124-186)

// Before (line ~128):
let content_type = data
    .get("content_type")
    .and_then(|v| v.as_str())
    .unwrap_or("unknown")
    .to_string();

// After:
let content_type = data
    .get("contentType")
    .and_then(|v| v.as_str())
    .unwrap_or("unknown")
    .to_string();

// Before (line ~151):
let content_format = data
    .get("content_format")
    .and_then(|v| v.as_str())
    .unwrap_or("markdown")
    .to_string();

// After:
let content_format = data
    .get("contentFormat")
    .and_then(|v| v.as_str())
    .unwrap_or("markdown")
    .to_string();

// Before (line ~173):
let trust_score = data
    .get("trust_score")
    .and_then(|v| v.as_f64())
    .unwrap_or(0.0);

// After:
let trust_score = data
    .get("trustScore")
    .and_then(|v| v.as_f64())
    .unwrap_or(0.0);

// Before (line ~178):
let estimated_minutes = data
    .get("estimated_minutes")
    .and_then(|v| v.as_u64())
    .map(|v| v as u32);

// After:
let estimated_minutes = data
    .get("estimatedMinutes")
    .and_then(|v| v.as_u64())
    .map(|v| v as u32);
```

Fields that are single words (`title`, `description`, `summary`, `tags`, `reach`, `content`) don't need changing — they're the same in snake_case and camelCase.

- [ ] **Step 2: Audit for other snake_case lookups in projection layer**

```bash
grep -rn 'get(".*_.*")' doorway/doorway-service/src/projection/ --include='*.rs' | grep -v test | grep -v '//'
```

Fix any additional snake_case lookups against the `data` JsonValue found.

- [ ] **Step 3: Compile and test**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo check && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -30
```

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/projection/
git commit -m "$(cat <<'EOF'
fix(doorway): use camelCase lookups in ContentProjection from_entry

The data field contains camelCase from elohim-storage's serde output,
but from_entry() was using snake_case lookups (content_format,
trust_score, estimated_minutes). This caused silent extraction failures
with defaults substituted for actual values.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Rename `contentBody.appId` → `slug` + rename `app_index` → `slug_index` (Rust)

This renames the HTML5 app identifier from `appId` to `slug` inside the content body JSON, and renames the in-memory index accordingly. Touches both Rust crates.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (load_app_index, lookup_app_blob_hash, app_index field)
- Modify: `doorway/doorway-service/src/cache/app_file_cache.rs` (extract functions, app_index field, load/resolve methods)
- Modify: `doorway/doorway-service/src/db/schemas/app_file_cache.rs` (MongoDB field + indexes)

- [ ] **Step 1: Rename in elohim-storage http.rs**

Rename the struct field:

```rust
// Before (line 147):
/// In-memory index: appId -> blobHash (avoids per-request SQLite scan)
app_index: Arc<RwLock<std::collections::HashMap<String, String>>>,

// After:
/// In-memory index: slug -> blobHash (avoids per-request SQLite scan)
slug_index: Arc<RwLock<std::collections::HashMap<String, String>>>,
```

In `load_app_index()` (~line 280), change the JSON key lookup:

```rust
// Before:
if let Some(app_id) = obj.get("appId").and_then(|v| v.as_str()) {

// After:
if let Some(slug) = obj.get("slug").and_then(|v| v.as_str()) {
```

And update all references: `self.app_index` → `self.slug_index`, local variable `app_id` → `slug` where it refers to the HTML5 app identifier (NOT the Holochain context).

In `lookup_app_blob_hash()` (~line 3088), same JSON key change:

```rust
// Before:
if let Some(content_app_id) = obj.get("appId").and_then(|v| v.as_str()) {

// After:
if let Some(slug) = obj.get("slug").and_then(|v| v.as_str()) {
```

Rename the function to `lookup_slug_blob_hash()` and update its caller in `handle_app_request()`.

In `handle_app_request()` (~line 2857), rename the local variable from the URL path:

```rust
// Before:
let (app_id, file_path) = match remainder.find('/') {

// After:
let (slug, file_path) = match remainder.find('/') {
```

Update all subsequent references in that function: `app_id` → `slug` (for the HTML5 app identifier), `self.app_index` → `self.slug_index`.

**Important**: Don't rename the URL path segment format or error messages yet — just the variable names and JSON key lookup. The URL is still `/apps/{slug}/...`.

Run `cargo check` to find all remaining references:

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | head -100
```

- [ ] **Step 2: Rename in doorway app_file_cache.rs**

Rename the struct field:

```rust
// Before (line 74):
app_index: Arc<RwLock<HashMap<String, String>>>,

// After:
slug_index: Arc<RwLock<HashMap<String, String>>>,
```

Rename both extract functions and update their JSON key lookups:

```rust
// Before (line 396):
fn extract_html5_app_id(doc: &crate::projection::document::ProjectedDocument) -> Option<String> {

// After:
fn extract_html5_slug(doc: &crate::projection::document::ProjectedDocument) -> Option<String> {
```

Inside both functions, change the content body JSON lookup:

```rust
// Before:
body.get("appId").and_then(|v| v.as_str())

// After:
body.get("slug").and_then(|v| v.as_str())
```

Remove the TODO comment at line 410:

```rust
// DELETE: // TODO: disambiguate as hAppId (Holochain) vs appId (HTML5) in future sprint.
```

Update comments to reflect the new naming:

```rust
// Before:
// The HTML5 app's appId is inside contentBody (a JSON string containing
// {appId, entryPoint, fallbackUrl}). data.appId is the Holochain app
// context (e.g., "lamad"), NOT the HTML5 app identifier.

// After:
// The HTML5 app's slug is inside contentBody (a JSON string containing
// {slug, entryPoint, fallbackUrl}). data.hAppId is the Holochain app
// context (e.g., "lamad"), NOT the HTML5 app identifier.
```

Rename all `app_index` → `slug_index`, and rename local `app_id` variables → `slug` where they refer to the HTML5 identifier.

Update `load_app_index()` → `load_slug_index()` and `resolve_blob_hash()` parameter name `app_id` → `slug`.

- [ ] **Step 3: Rename MongoDB field in app_file_cache schema**

```rust
// File: doorway/doorway-service/src/db/schemas/app_file_cache.rs

// Before (line 34):
/// The app's content ID (EPR identifier)
pub app_id: String,

// After:
/// Human-readable URL slug declared by the content
pub slug: String,
```

Update the `_id` format comment:

```rust
// Before (line 31):
/// MongoDB document ID (format: "{app_id}:{file_path}:{blob_hash}")

// After:
/// MongoDB document ID (format: "{slug}:{file_path}:{blob_hash}")
```

Update index definitions:

```rust
// Before (lines 112, 121):
doc! { "app_id": 1 },
    IndexOptions::builder().name("app_id_index".to_string()).build(),
doc! { "app_id": 1, "blob_hash": 1 },
    IndexOptions::builder().name("app_id_blob_hash_index".to_string()).build(),

// After:
doc! { "slug": 1 },
    IndexOptions::builder().name("slug_index".to_string()).build(),
doc! { "slug": 1, "blob_hash": 1 },
    IndexOptions::builder().name("slug_blob_hash_index".to_string()).build(),
```

- [ ] **Step 4: Fix all compiler errors in doorway-service**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | head -100
```

The `app_file_cache.rs` functions that use `cache_key()`, `in_flight_key()`, `invalidate_app()` etc. all reference `app_id` — rename to `slug`:

```rust
// cache_key: Before:
format!("{}:{}:{}", app_id, file_path, blob_hash)
// After:
format!("{}:{}:{}", slug, file_path, blob_hash)

// in_flight_key: Before:
format!("apps:{}:{}", app_id, file_path)
// After:
format!("apps:{}:{}", slug, file_path)
```

In `routes/apps.rs`, rename local variables from `app_id` → `slug` where they represent the HTML5 identifier:

```rust
// Before (in handle_app_request):
let (app_id, file_path) = match parse_app_path(path) {

// After:
let (slug, file_path) = match parse_app_path(path) {
```

Update `parse_app_path()` return comment but keep the function name (it parses the URL path, which uses slugs).

- [ ] **Step 5: Compile and test both crates**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -30
```

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/http.rs doorway/doorway-service/src/
git commit -m "$(cat <<'EOF'
refactor: rename contentBody.appId → slug, app_index → slug_index

HTML5 app identifier is now "slug" (content-declared, WordPress model).
Renamed in-memory index, extract functions, MongoDB schema, and cache
keys. Resolves the TODO in app_file_cache.rs.

Three distinct identifiers: hAppId (Holochain context), slug (URL path),
blobHash (content address).

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Regenerate TypeScript types + fix storage-client-ts

**Files:**
- Regenerate: `elohim/sdk/storage-client-ts/src/generated/*.ts`
- Modify: `elohim/sdk/storage-client-ts/src/types.ts`
- Modify: `elohim/sdk/storage-client-ts/src/client.ts`

- [ ] **Step 1: Regenerate types from Rust**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -20
```

This regenerates TypeScript types in `elohim/sdk/storage-client-ts/src/generated/`. The generated types will now have `h_app_id` (ts-rs preserves Rust field names, not serde renames).

- [ ] **Step 2: Rename in StorageConfig**

```typescript
// File: elohim/sdk/storage-client-ts/src/types.ts

// Before (line 8):
  appId: string;

// After:
  hAppId: string;
```

Update the JSDoc comment accordingly.

- [ ] **Step 3: Rename in StorageClient**

```typescript
// File: elohim/sdk/storage-client-ts/src/client.ts

// All URL construction (lines 118, 128, 138, 153, 170):
// Before:
`/sync/v1/${this.config.appId}/docs`

// After:
`/sync/v1/${this.config.hAppId}/docs`
```

Apply the same change to all 5 sync API methods: `listDocuments`, `getDocument`, `getHeads`, `getChangesSince`, `applyChanges`.

- [ ] **Step 4: Type-check**

```bash
cd elohim/sdk/storage-client-ts && npx tsc --noEmit 2>&1 | head -30
```

Fix any remaining type errors.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/storage-client-ts/
git commit -m "$(cat <<'EOF'
refactor(sdk): regenerate types, rename appId → hAppId in storage client

Generated types now have h_app_id. StorageConfig and StorageClient
sync API paths updated to use hAppId.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: TypeScript `hAppId` rename (SDK, library, app configs)

Rename `appId` → `hAppId` in all TypeScript files where it represents the Holochain app context.

**Files:**
- Modify: `elohim/sdk/src/connection.ts` (ConnectionState, ConnectionConfig)
- Modify: `elohim/sdk/src/types.ts` (ConnectionConfig, DEFAULT_APP_ID)
- Modify: `elohim/holochain/rna/typescript/src/config.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/models/holochain.model.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/client/types.ts`
- Modify: `app/elohim-app/src/environments/environment.types.ts`
- Modify: `app/elohim-app/src/environments/environment.ts`
- Modify: `app/elohim-app/src/environments/environment.alpha.ts`
- Modify: `app/elohim-app/src/environments/environment.staging.ts`
- Modify: `app/elohim-app/src/environments/environment.prod.ts`
- Modify: `app/elohim-app/src/app/app.config.ts`

- [ ] **Step 1: Rename in SDK connection types**

```typescript
// File: elohim/sdk/src/connection.ts

// ConnectionState (line 21):
// Before:
appId: string | null;
// After:
hAppId: string | null;

// Constructor (line 44):
// Before:
appId: DEFAULT_APP_ID,
// After:
hAppId: DEFAULT_APP_ID,

// App matching (line 83):
// Before:
const appInfo = apps.find((app: AppInfo) => app.installed_app_id === this.config.appId);
// After:
const appInfo = apps.find((app: AppInfo) => app.installed_app_id === this.config.hAppId);

// Return state (line 166):
// Before:
appId: this.config.appId ?? null,
// After:
hAppId: this.config.hAppId ?? null,
```

```typescript
// File: elohim/sdk/src/types.ts

// ConnectionConfig (line 2979):
// Before:
appId?: string;
// After:
hAppId?: string;
```

- [ ] **Step 2: Rename in RNA config**

```typescript
// File: elohim/holochain/rna/typescript/src/config.ts

// ConnectionConfig (line 80):
// Before:
appId: string;
// After:
hAppId: string;
```

- [ ] **Step 3: Rename in elohim-library models**

```typescript
// File: app/elohim-library/projects/elohim-service/src/models/holochain.model.ts

// HolochainClientConfig (line 223):
// Before:
appId: string;
// After:
hAppId: string;

// File: app/elohim-library/projects/elohim-service/src/client/types.ts

// HolochainConnection (line 112):
// Before:
appId: string;
// After:
hAppId: string;
```

- [ ] **Step 4: Rename in environment configs**

```typescript
// File: app/elohim-app/src/environments/environment.types.ts (line 67):
// Before:
holochainAppId?: string;
// After:
holochainHAppId?: string;
```

Update all environment files that reference this:

```typescript
// Files: environment.ts, environment.alpha.ts, environment.staging.ts, environment.prod.ts
// Before:
holochainAppId: 'elohim',
// After:
holochainHAppId: 'elohim',
```

- [ ] **Step 5: Rename in app.config.ts**

```typescript
// File: app/elohim-app/src/app/app.config.ts (lines 36-38):
// Before:
holochain: environment.client?.holochainAppId
    ? {
        appId: environment.client.holochainAppId,

// After:
holochain: environment.client?.holochainHAppId
    ? {
        hAppId: environment.client.holochainHAppId,
```

- [ ] **Step 6: Find and fix remaining references**

```bash
grep -rn 'appId\|app_id' app/elohim-app/src/ elohim/sdk/src/ elohim/holochain/rna/typescript/src/ app/elohim-library/ --include='*.ts' | grep -v node_modules | grep -v '.spec.' | grep -v 'slug' | grep -v 'contentBody'
```

Fix any remaining references that represent the Holochain app context. **Do NOT rename** references that represent:
- `installed_app_id` (Holochain conductor naming)
- `appId` inside content body (being renamed to `slug` in Task 7)

- [ ] **Step 7: Commit**

```bash
git add elohim/sdk/src/ elohim/holochain/rna/typescript/ app/elohim-library/ app/elohim-app/src/environments/ app/elohim-app/src/app/app.config.ts
git commit -m "$(cat <<'EOF'
refactor(ts): rename appId → hAppId for Holochain app context

Updated SDK ConnectionConfig/ConnectionState, RNA config,
elohim-library HolochainClientConfig, environment types, and
app.config.ts. All now use hAppId to distinguish from HTML5 slug.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: TypeScript slug rename (iframe-renderer, html5-app plugin, service worker)

Rename `appId` → `slug` in TypeScript files where it represents the HTML5 app identifier.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/content-io/plugins/html5-app/html5-app-format.plugin.ts`
- Modify: `app/elohim-app/src/apps-sw.ts`

- [ ] **Step 1: Rename in iframe-renderer**

```typescript
// File: app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts

// Interface (lines 14-21):
// Before:
export interface Html5AppContent {
  appId: string;
  entryPoint: string;
  fallbackUrl?: string;
}

// After:
export interface Html5AppContent {
  slug: string;
  entryPoint: string;
  fallbackUrl?: string;
}

// buildHtml5AppUrl (line 160):
// Before:
const { appId, entryPoint } = content;
// After:
const { slug, entryPoint } = content;

// URL construction (line 167):
// Before:
return `${doorwayUrl}/apps/${appId}/${entryPoint}`;
// After:
return `${doorwayUrl}/apps/${slug}/${entryPoint}`;
```

- [ ] **Step 2: Rename in html5-app-format.plugin.ts**

```typescript
// Content Structure doc comment (line 39):
// Before:
//  *     appId: 'evolution-of-trust',
// After:
//  *     slug: 'evolution-of-trust',

// Import method (lines 96-101):
// Before:
const appId = this.slugify(filename);
return {
  content: {
    appId,
    entryPoint: 'index.html',
  } as Html5AppContent,

// After:
const slug = this.slugify(filename);
return {
  content: {
    slug,
    entryPoint: 'index.html',
  } as Html5AppContent,

// Export method (lines 144-146):
// Before:
appId: content.appId,
// After:
slug: content.slug,

// Validation (lines 210-221):
// Before:
private validateAppId(content: Partial<Html5AppContent>, errors: ValidationError[]): void {
    if (!content.appId || typeof content.appId !== 'string') {
      errors.push({
        code: 'MISSING_APP_ID',
        message: 'Missing required field: appId (string)',
      });
    } else if (!/^[a-z0-9-]+$/.test(content.appId)) {

// After:
private validateSlug(content: Partial<Html5AppContent>, errors: ValidationError[]): void {
    if (!content.slug || typeof content.slug !== 'string') {
      errors.push({
        code: 'MISSING_SLUG',
        message: 'Missing required field: slug (string)',
      });
    } else if (!/^[a-z0-9-]+$/.test(content.slug)) {

// Format detection (line 322):
// Before:
if (typeof obj['appId'] === 'string' && typeof obj['entryPoint'] === 'string') {
// After:
if (typeof obj['slug'] === 'string' && typeof obj['entryPoint'] === 'string') {
```

Update the caller of `validateAppId` to call `validateSlug` instead.

- [ ] **Step 3: Rename in apps-sw.ts**

```typescript
// File: app/elohim-app/src/apps-sw.ts

// Delivery cache (line 133-134):
// Before:
/** Probe results cached per app_id — cleared on invalidation or new blob_hash */
const deliveryCache = new Map<string, DeliveryInfo>();

// After:
/** Probe results cached per slug — cleared on invalidation or new blob_hash */
const deliveryCache = new Map<string, DeliveryInfo>();

// probeCapability (lines 136-160):
// Rename parameter and all references: appId → slug
// Before:
async function probeCapability(appId: string): Promise<DeliveryInfo> {
  const cached = deliveryCache.get(appId);
  if (cached) return cached;
  const resp = await fetch(`/apps/${appId}/_capability`, { method: 'HEAD' });
  ...
  deliveryCache.set(appId, info);

// After:
async function probeCapability(slug: string): Promise<DeliveryInfo> {
  const cached = deliveryCache.get(slug);
  if (cached) return cached;
  const resp = await fetch(`/apps/${slug}/_capability`, { method: 'HEAD' });
  ...
  deliveryCache.set(slug, info);

// Fetch handler path parsing (lines 168-169):
// Before:
const appId = pathParts[0];
// After:
const slug = pathParts[0];

// extractZip (lines 253-274):
// Rename parameter: appId → slug
// Before:
async function extractZip(cache: Cache, appId: string, blobHash: string): Promise<void> {
  const blobUrl = blobHash ? `/blob/${blobHash}` : `/apps/${appId}/`;
  ...
  await cache.put(
    new Request(`${self.location.origin}/apps/${appId}/${path}`),
    response,
  );

// After:
async function extractZip(cache: Cache, slug: string, blobHash: string): Promise<void> {
  const blobUrl = blobHash ? `/blob/${blobHash}` : `/apps/${slug}/`;
  ...
  await cache.put(
    new Request(`${self.location.origin}/apps/${slug}/${path}`),
    response,
  );

// Invalidation handler (lines 315-327):
// Before:
const { type, appId } = event.data;
if (type === 'invalidate' && appId) {
  const prefix = `/apps/${appId}/`;
  ...
  deliveryCache.delete(appId);
  console.log(`[apps-sw] invalidated ${toDelete.length} files for ${appId}`);

// After:
const { type, slug } = event.data;
if (type === 'invalidate' && slug) {
  const prefix = `/apps/${slug}/`;
  ...
  deliveryCache.delete(slug);
  console.log(`[apps-sw] invalidated ${toDelete.length} files for ${slug}`);
```

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/lamad/ app/elohim-app/src/apps-sw.ts
git commit -m "$(cat <<'EOF'
refactor(app): rename appId → slug for HTML5 app content identifier

Html5AppContent.appId → .slug, validation/format detection updated,
service worker uses slug terminology. Content body now stores
{slug, entryPoint, fallbackUrl}.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Seeder updates

Update the seeder pipeline to use the new field names.

**Files:**
- Modify: `genesis/seeder/src/doorway-client.ts`
- Modify: `genesis/seeder/src/seed.ts`

- [ ] **Step 1: Update doorway-client.ts registerApp()**

```typescript
// File: genesis/seeder/src/doorway-client.ts

// Function signature (line 540):
// Before:
async registerApp(
    appId: string,

// After:
async registerApp(
    slug: string,

// Dry run log (line 546):
// Before:
console.log(`[DRY RUN] Would register app: ${appId} -> ${blobHash}`);
// After:
console.log(`[DRY RUN] Would register app: ${slug} -> ${blobHash}`);

// Request body (lines 555-560):
// Before:
body: JSON.stringify({
    appId: appId,
    blobHash: blobHash,
    entryPoint: entryPoint,
    fallbackUrl: fallbackUrl,
}),

// After:
body: JSON.stringify({
    slug: slug,
    blobHash: blobHash,
    entryPoint: entryPoint,
    fallbackUrl: fallbackUrl,
}),
```

- [ ] **Step 2: Update seed.ts content construction**

```typescript
// File: genesis/seeder/src/seed.ts

// Comment (line 761):
// Before:
// Exception: html5-app keeps original content object (appId, entryPoint, fallbackUrl)
// because the renderer needs that to build the /apps/{appId}/{entryPoint} URL
// After:
// Exception: html5-app keeps original content object (slug, entryPoint, fallbackUrl)
// because the renderer needs that to build the /apps/{slug}/{entryPoint} URL

// HTML5 app blob hash map type (line 1136):
// Before:
const html5AppBlobHashes = new Map<string, { hash: string; appId: string; entryPoint: string }>();
// After:
const html5AppBlobHashes = new Map<string, { hash: string; slug: string; entryPoint: string }>();

// Content extraction (line 1169):
// Before:
const appId = contentObj?.appId as string || concept.id;
// After:
const slug = contentObj?.slug as string || concept.id;

// Map insert (lines 1172-1176):
// Before:
html5AppBlobHashes.set(concept.id, {
    hash: processed.blobMetadata.hash,
    appId,
    entryPoint,
});
// After:
html5AppBlobHashes.set(concept.id, {
    hash: processed.blobMetadata.hash,
    slug,
    entryPoint,
});

// Log (line 1178):
// Before:
console.log(`   ✅ ${concept.id}: ${uploadResult.cached ? 'already cached' : 'uploaded'} (appId: ${appId})`);
// After:
console.log(`   ✅ ${concept.id}: ${uploadResult.cached ? 'already cached' : 'uploaded'} (slug: ${slug})`);

// Metadata enrichment (line 1224):
// Before:
meta.appId = appInfo.appId;
// After:
meta.slug = appInfo.slug;
```

- [ ] **Step 3: Check for seed JSON fixtures**

```bash
grep -rn '"appId"' genesis/seeder/ genesis/data/ --include='*.json' --include='*.ts' | grep -v node_modules | grep -v '.d.ts'
```

Update any JSON fixtures that have `"appId"` inside content body objects to use `"slug"`.

- [ ] **Step 4: Type-check seeder**

```bash
cd genesis/seeder && npx tsc --noEmit 2>&1 | head -30
```

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/
git commit -m "$(cat <<'EOF'
refactor(seeder): rename appId → slug in content construction

registerApp() now sends slug field. Seed data construction and
metadata enrichment updated. Consistent with storage/doorway rename.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Phase 1 full verification

Run all quality gates to confirm nothing is broken.

**Files:** None (verification only)

- [ ] **Step 1: Rust — elohim-storage**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -30
```

- [ ] **Step 2: Rust — doorway-service**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -30
```

- [ ] **Step 3: Rust — clippy**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -20
cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -20
```

- [ ] **Step 4: Rust — formatting**

```bash
cd elohim/elohim-storage && cargo fmt --check
cd doorway/doorway-service && cargo fmt --check
```

- [ ] **Step 5: TypeScript — type check**

```bash
cd app/elohim-app && npx tsc --noEmit 2>&1 | head -30
```

- [ ] **Step 6: Grep for leftover appId references (excluding installed_app_id)**

```bash
grep -rn '\bapp_id\b' elohim/elohim-storage/src/ doorway/doorway-service/src/ --include='*.rs' | grep -v installed_app_id | grep -v test | grep -v '//'
grep -rn '\bappId\b' app/elohim-app/src/ app/elohim-library/ --include='*.ts' | grep -v node_modules | grep -v installed_app_id | grep -v '.spec.' | grep -v 'hAppId'
```

If any remain, determine whether they're Holochain context (→ `h_app_id`/`hAppId`) or HTML5 slug (→ `slug`) and fix.

- [ ] **Step 7: Fix any issues found, then commit**

If fixes were needed:
```bash
git add -A
git commit -m "$(cat <<'EOF'
fix: address remaining appId references from Phase 1 rename

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2: CID URL Support

### Task 10: Dual-path URL resolution (elohim-storage)

Add support for `/apps/{blob_hash}/file` as an alternate URL path alongside `/apps/{slug}/file`. Add `X-Content-Address` header to all app responses.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Add blob_hash pattern detection helper**

Add near the `handle_app_request` function:

```rust
/// Check if an identifier is a content address (blob hash) rather than a slug
fn is_content_address(identifier: &str) -> bool {
    identifier.starts_with("sha256-") && identifier.len() > 10
}
```

- [ ] **Step 2: Modify handle_app_request for dual-path resolution**

In `handle_app_request()`, after parsing the path segment, add CID detection:

```rust
// After parsing (slug, file_path) from path:

// Resolve blob_hash: direct if CID, lookup if slug
let (resolved_slug, blob_hash) = if is_content_address(slug) {
    // CID-based URL: blob_hash IS the identifier, no slug available
    (None, Some(slug.to_string()))
} else {
    // Slug-based URL: resolve via slug_index
    let cached_blob_hash = {
        let index = self.slug_index.read().await;
        index.get(slug).cloned()
    };
    (Some(slug.to_string()), cached_blob_hash)
};
```

Replace the existing `cached_blob_hash` lookup with this dual-path version.

For the slow path (cache miss), update the DB lookup call:

```rust
// Before:
None => match self.lookup_slug_blob_hash(slug).await? {

// After (only look up if we have a slug, not a CID):
None => {
    if is_content_address(slug) {
        // CID was provided but blob not found — 404
        return Ok(not_found_response(format!("Blob not found: {}", slug)));
    }
    match self.lookup_slug_blob_hash(slug).await? {
```

- [ ] **Step 3: Add X-Content-Address header to all app responses**

In every `Response::builder()` chain within `handle_app_request()`, add the header. There are 3 success response locations (cache HIT, HIT-COALESCED, MISS):

```rust
// Add to each response builder:
.header("X-Content-Address", &blob_hash)
```

If the slug was provided, also add:

```rust
.header("X-Content-Slug", resolved_slug.as_deref().unwrap_or(""))
```

- [ ] **Step 4: Compile and test**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "$(cat <<'EOF'
feat(storage): dual-path URL resolution for /apps/ routes

Both /apps/{slug}/file and /apps/{sha256-hash}/file now serve content.
CID-based URLs skip slug_index lookup. All app responses include
X-Content-Address and X-Content-Slug headers.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: Dual-path URL resolution (doorway)

Mirror the dual-path support in doorway's app routes and cache.

**Files:**
- Modify: `doorway/doorway-service/src/routes/apps.rs`
- Modify: `doorway/doorway-service/src/cache/app_file_cache.rs`

- [ ] **Step 1: Add CID detection to doorway routes**

```rust
// File: doorway/doorway-service/src/routes/apps.rs

// Add helper (near parse_app_path):
fn is_content_address(identifier: &str) -> bool {
    identifier.starts_with("sha256-") && identifier.len() > 10
}
```

In `handle_app_request()`, after parsing, short-circuit cache resolution for CIDs:

```rust
// After: let (slug, file_path) = match parse_app_path(path) { ... };

if let Some(ref cache) = state.app_file_cache {
    let blob_hash = if is_content_address(slug) {
        // CID-based URL: identifier IS the blob_hash
        Some(slug.to_string())
    } else {
        cache.resolve_blob_hash(slug).await
    };
    // ... rest of cache-first flow unchanged
}
```

- [ ] **Step 2: Short-circuit in resolve_blob_hash**

```rust
// File: doorway/doorway-service/src/cache/app_file_cache.rs

// In resolve_blob_hash():
pub async fn resolve_blob_hash(&self, identifier: &str) -> Option<String> {
    // Short-circuit: if identifier is already a content address, return it directly
    if identifier.starts_with("sha256-") && identifier.len() > 10 {
        return Some(identifier.to_string());
    }

    // Fast path: check slug_index
    // ... existing code ...
}
```

- [ ] **Step 3: Forward X-Content-Address header**

In `fetch_and_cache()` and `forward_app_request_with_header()`, read the `X-Content-Address` header from the storage response and include it in the doorway response:

```rust
// In build_app_response, add optional blob_hash parameter:
fn build_app_response(
    data: &[u8],
    content_type: &str,
    cache_status: &str,
    blob_hash: Option<&str>,
    slug: Option<&str>,
) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .header("Cross-Origin-Embedder-Policy", "credentialless")
        .header("X-Cache", cache_status)
        .header("Cache-Control", "public, max-age=3600");

    if let Some(hash) = blob_hash {
        builder = builder.header("X-Content-Address", hash);
    }
    if let Some(s) = slug {
        builder = builder.header("X-Content-Slug", s);
    }

    builder.body(Full::new(Bytes::from(data.to_vec()))).unwrap()
}
```

Update all callers to pass the blob_hash and slug.

- [ ] **Step 4: Compile and test**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo check
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/
git commit -m "$(cat <<'EOF'
feat(doorway): dual-path URL resolution + content address headers

Both /apps/{slug}/file and /apps/{sha256-hash}/file now work.
resolve_blob_hash() short-circuits for CID identifiers. Responses
include X-Content-Address and X-Content-Slug headers.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: Service Worker CID caching

Update the service worker to cache entries by content address (blob_hash) instead of slug, enabling immutable cache keys.

**Files:**
- Modify: `app/elohim-app/src/apps-sw.ts`

- [ ] **Step 1: Read X-Content-Address header and cache by CID**

In `handleAppFetch()`, after getting the response, read the content address header and use it for the cache key:

```typescript
// In handleAppFetch(), after getting the response from fetch or cache:

async function handleAppFetch(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const pathParts = url.pathname.replace('/apps/', '').split('/');
  const identifier = pathParts[0]; // Could be slug or blob_hash
  const filePath = pathParts.slice(1).join('/');

  const cache = await caches.open(CACHE_NAME);

  // If identifier is already a CID, check cache directly
  const isContentAddress = identifier.startsWith('sha256-');

  // Try cache under CID key first (if we know the blob_hash)
  const capability = await probeCapability(identifier);
  const blobHash = capability.blobHash;

  if (blobHash) {
    const cidKey = new Request(`${self.location.origin}/apps/${blobHash}/${filePath}`);
    const cached = await cache.match(cidKey);
    if (cached) return cached;
  }

  // Also check under the original URL (backwards compat during transition)
  const cached = await cache.match(request);
  if (cached) return cached;

  // Fetch from network
  const response = await fetch(request);
  if (!response.ok) return response;

  // Read content address from response header
  const contentAddress = response.headers.get('X-Content-Address') || blobHash;

  // Clone response for caching
  const responseToCache = response.clone();

  // Cache under CID key (immutable) if we have a content address
  if (contentAddress) {
    const cidKey = new Request(`${self.location.origin}/apps/${contentAddress}/${filePath}`);
    await cache.put(cidKey, responseToCache);
  } else {
    // Fallback: cache under original URL
    await cache.put(request, responseToCache);
  }

  return response;
}
```

- [ ] **Step 2: Update extractZip to cache by CID**

```typescript
// In extractZip():
async function extractZip(cache: Cache, slug: string, blobHash: string): Promise<void> {
  const blobUrl = blobHash ? `/blob/${blobHash}` : `/apps/${slug}/`;
  const resp = await fetch(blobUrl);
  if (!resp.ok) return;

  const data = await resp.arrayBuffer();
  const zip = await JSZip.loadAsync(data);

  // Use blobHash for cache keys (content-addressed, immutable)
  const cachePrefix = blobHash || slug;

  for (const [path, file] of Object.entries(zip.files)) {
    if (file.dir) continue;
    const content = await file.async('arraybuffer');
    const contentType = guessContentType(path);
    const response = new Response(content, {
      headers: { 'Content-Type': contentType },
    });
    await cache.put(
      new Request(`${self.location.origin}/apps/${cachePrefix}/${path}`),
      response,
    );
  }
}
```

- [ ] **Step 3: Update invalidation to clear both slug and CID entries**

```typescript
// In the BroadcastChannel handler:
channel.onmessage = async (event: MessageEvent) => {
  const { type, slug, blobHash } = event.data;
  if (type === 'invalidate' && (slug || blobHash)) {
    const cache = await caches.open(CACHE_NAME);
    const keys = await cache.keys();

    // Build prefixes to clear
    const prefixes: string[] = [];
    if (slug) prefixes.push(`/apps/${slug}/`);
    if (blobHash) prefixes.push(`/apps/${blobHash}/`);

    const toDelete = keys.filter((req) => {
      const pathname = new URL(req.url).pathname;
      return prefixes.some((prefix) => pathname.startsWith(prefix));
    });
    await Promise.all(toDelete.map((key) => cache.delete(key)));

    if (slug) deliveryCache.delete(slug);
    if (blobHash) deliveryCache.delete(blobHash);

    console.log(
      `[apps-sw] invalidated ${toDelete.length} files for ${slug || blobHash}`,
    );
  }
};
```

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/apps-sw.ts
git commit -m "$(cat <<'EOF'
feat(sw): cache HTML5 app files by content address (CID)

Service worker now reads X-Content-Address header and caches entries
under /apps/{blobHash}/path instead of /apps/{slug}/path. This makes
cache keys immutable — re-seeded content with a new hash gets a fresh
cache entry. Invalidation clears both slug and CID key patterns.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 13: A2O regression scenarios + Phase 2 verification

Add Gherkin scenarios documenting the new behavior and run final verification.

**Files:**
- Create: `genesis/a2o/features/delivery/content-addressing.feature`

- [ ] **Step 1: Write regression scenarios**

```gherkin
# File: genesis/a2o/features/delivery/content-addressing.feature

Feature: Content-addressed delivery
  As a learner visiting an HTML5 app
  I want content served by both slug and content address
  So that my browser cache stays valid across versions

  Background:
    Given the doorway is connected to elohim-storage
    And an HTML5 app with slug "evolution-of-trust" is seeded
    And the app's blob hash is "sha256-abc123"

  Scenario: Slug URL serves content with content address header
    When I request "/apps/evolution-of-trust/index.html"
    Then the response status is 200
    And the response includes header "X-Content-Address" with value "sha256-abc123"
    And the response includes header "X-Content-Slug" with value "evolution-of-trust"

  Scenario: CID URL serves same content without slug lookup
    When I request "/apps/sha256-abc123/index.html"
    Then the response status is 200
    And the response includes header "X-Content-Address" with value "sha256-abc123"
    And the response body matches the slug URL response

  Scenario: Service worker caches by content address
    Given the service worker is active
    When I navigate to "/apps/evolution-of-trust/index.html"
    Then the service worker caches the response under "/apps/sha256-abc123/index.html"
    When I navigate to "/apps/sha256-abc123/index.html"
    Then the response is served from the service worker cache

  Scenario: Re-seeded content with new CID invalidates old mapping
    Given the service worker has cached files under "/apps/sha256-abc123/"
    When the app is re-seeded with blob hash "sha256-def456"
    And I navigate to "/apps/evolution-of-trust/index.html"
    Then the response includes header "X-Content-Address" with value "sha256-def456"
    And the old cache entries under "/apps/sha256-abc123/" are invalidated
```

- [ ] **Step 2: Run Rust tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -20
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20
```

- [ ] **Step 3: Run clippy + fmt**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -20
cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -20
cd elohim/elohim-storage && cargo fmt --check
cd doorway/doorway-service && cargo fmt --check
```

- [ ] **Step 4: TypeScript type check**

```bash
cd app/elohim-app && npx tsc --noEmit 2>&1 | head -30
```

- [ ] **Step 5: Final leftover grep**

```bash
# Should find ZERO results (excluding installed_app_id, comments, test files):
grep -rn '\bapp_id\b' elohim/elohim-storage/src/ doorway/doorway-service/src/ --include='*.rs' | grep -v installed_app_id | grep -v '^\s*//' | grep -v '#\[' | grep -v test
```

- [ ] **Step 6: Commit**

```bash
git add genesis/a2o/features/delivery/content-addressing.feature
git commit -m "$(cat <<'EOF'
feat(a2o): add content-addressing regression scenarios

Covers slug URL + CID URL serving, X-Content-Address header,
service worker CID caching, and re-seed invalidation.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Post-Sprint

After all tasks complete:
1. Wipe SQLite database
2. Wipe MongoDB `app_file_cache` and `projected_entries` collections (indexes changed)
3. Reseed with `pnpm run hc:start:seed` from `app/elohim-app`
4. Verify projected documents have `hAppId` and content body has `slug`
5. Verify both `/apps/{slug}/index.html` and `/apps/{sha256-hash}/index.html` serve content
6. Verify `X-Content-Address` header in browser dev tools
