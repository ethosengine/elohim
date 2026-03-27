# Content Pipeline: Genesis to Screen

How content defined in genesis gets uploaded, persisted, distributed, retrieved, cached, and displayed by lamad.

## Stage 1: Authoring

Content starts as markdown in `genesis/docs/content/`. The elohim-import CLI (`/import-content` skill) transforms markdown into structured JSON:

```
genesis/docs/content/elohim-protocol/autonomous_entity/worker/README.md
    ↓  elohim-import CLI (parses frontmatter, extracts metadata, assigns ID)
genesis/data/lamad/content/autonomous-entity-worker-readme.json
```

A content JSON file looks like:

```json
{
  "id": "autonomous-entity-worker-readme",
  "title": "The Worker's Perspective",
  "contentType": "concept",
  "contentFormat": "markdown",
  "content": "# The Worker's Perspective\n\nIn traditional organizations...",
  "description": "Understanding organizational ownership from the worker's viewpoint",
  "tags": ["autonomous-entity", "ownership"],
  "metadata": { "sourcePath": "autonomous_entity/worker/README.md" }
}
```

Paths are authored directly as JSON in `genesis/data/lamad/paths/`. They define a hierarchy: chapters → modules → sections → conceptIds (references to content IDs).

## Stage 2: Seeding

`seed-sqlite.ts` reads JSON files and POSTs to elohim-storage's HTTP API.

### Content transform (`seed-sqlite.ts:578-613`)

```
ConceptJson  →  CreateContentInput (schema-generated type)
├─ content (string|object)  →  contentBody (string)
├─ metadata + estimatedMinutes + thumbnailUrl + relatedNodeIds  →  metadata (parsed object)
├─ contentFormat  →  contentFormat (normalized via mapping table)
└─ reach  →  reach (from account packages or defaults to 'commons')
```

### Path transform (`seed-sqlite.ts:769-798`)

```
PathJson  →  CreateContentInput (contentType='path', contentFormat='epr-composite')
├─ chapters/modules/sections/conceptIds  →  contentBody: JSON { sections: [...] }
│   Each section has items: [{ ref: "epr:{conceptId}", role: "step", title: "..." }]
└─ pathType, difficulty, thumbnailUrl, etc.  →  metadata (parsed object)
```

### Blob upload (Phase 0, before content seeding)

- HTML5 apps (ZIPs) and thumbnails uploaded via `PUT /blob/{hash}`
- `blob-manager.ts` computes CID + SHA256 hash
- `blobHash` field in content references the uploaded blob

### API call (`seed-sqlite.ts:804-825`)

```
POST ${STORAGE_URL}/db/content/bulk
Content-Type: application/json
Body: [ { id, title, contentBody, metadata: {...}, contentType, contentFormat, reach, tags, ... } ]

Response: { inserted: 56, skipped: 0, errors: [] }
```

Sent in batches of 500.

## Stage 3: Storage Persistence

elohim-storage receives the request at `http.rs:1947`.

### API boundary (`views.rs:993-1045`)

```
HTTP JSON (camelCase)           Rust CreateContentInputView
  contentBody: "# markdown"  →   content_body: Option<String>
  metadata: { key: "val" }   →   metadata: Option<JsonVal>     // parsed serde_json::Value
  contentType: "concept"     →   content_type: String
```

### DB write (`views.rs:1028-1045` → `content_diesel.rs:318-390`)

```
CreateContentInputView.metadata: JsonVal
    → serialize_json_opt()
    → CreateContentInput.metadata_json: String   // re-serialized for SQLite TEXT column

diesel::insert_into(content::table).values(&new_content).execute()
```

### SQLite schema (`migrations/.../up.sql:37-61`)

```sql
content (
  id TEXT PRIMARY KEY,
  app_id TEXT DEFAULT 'lamad',
  title TEXT NOT NULL,
  content_body TEXT,           -- inline markdown/JSON
  content_type TEXT DEFAULT 'concept',
  content_format TEXT DEFAULT 'markdown',
  blob_hash TEXT,              -- reference to blob store
  blob_cid TEXT,               -- CIDv1 content address
  metadata_json TEXT,          -- serialized JSON string
  reach TEXT DEFAULT 'public',
  ...
)
```

Tags go to a separate `content_tags` table.

## Stage 4: Doorway Serving

Doorway is the gateway between browsers and storage. Two paths:

### Path A: DB proxy (most common for seeder + Angular)

```
GET /db/content/{id}  →  doorway (db.rs:35)  →  forwards to elohim-storage  →  returns ContentView
```

Pure pass-through. No caching, no transformation.

### Path B: Projection cache (used by Angular in some contexts)

```
GET /api/v1/cache/Content/{id}  →  doorway (api.rs:191)
    ├─ Tier 1: Projection store (fast)
    ├─ Tier 2: Conductor/DHT fallback (authoritative)
    └─ Returns: ResolutionResult<JSON>
```

### Response format

Both paths return `ContentView` (`views.rs:131-177`):

```
Content row (metadata_json: String)
    → parse_json_opt()           // String → JsonVal
    → ContentView { metadata: Option<JsonVal> }   // camelCase JSON response
    → serde_json::to_string()    // wire format
```

TypeScript receives `metadata` as a parsed object — never needs `JSON.parse()`.

## Stage 5: Angular Retrieval

### Connection strategy (`doorway-connection-strategy.ts:226-252`)

| Context | Base URL | Route |
|---------|----------|-------|
| Che dev | `window.location.origin` (proxy) | `/db/content/{id}` |
| Production | `https://alpha.elohim.host` | `/db/content/{id}` |
| Tauri desktop | `http://localhost:8090` | `/db/content/{id}` |

### ContentService.getContent() (`content.service.ts:305-348`)

```
ElohimClient.get('content', id)
    → HTTP GET /db/content/{id}
    → response: RawContentData
    → detect blob reference in contentBody (sha256: or bafk prefix)
    → if blob: fetchBlobContent(blobCid) via Helia P2P or HTTP fallback
    → transformContent(data) → ContentNode
```

## Stage 6: Content Transformation

### transformContent() (`content.service.ts:652-675`)

```
RawContentData                          ContentNode
  contentBody: "# Markdown..."    →    content: "# Markdown..."  (string for md)
  contentBody: '{"appId":...}'    →    content: { appId: ... }   (parsed for sophia/html5-app)
  metadata: { key: "val" }        →    metadata: { key: "val" }  (pass-through)
  thumbnailUrl: "/blob/sha256-"   →    thumbnailUrl: "https://alpha.../api/blob/sha256-"  (resolved)
```

Key transformations:

- **parseContentBody()** — JSON-parses structured formats (sophia, perseus, html5-app), leaves markdown as string
- **resolveBlobUrl()** — converts `/blob/sha256-...` to full URL via `StorageClientService.getBlobUrl()`

## Stage 7: Rendering

### Content viewer (`content-viewer.component.ts:298-368`)

```
Route param :resourceId
    → ContentService.getContent(id)
    → ContentNode received
    → RendererRegistry.getRenderer(node)  // selects by contentFormat
    → ViewContainerRef.createComponent(rendererComponent)
    → rendererRef.setInput('node', contentNode)
```

### Renderer selection (`renderer-registry.service.ts`)

| contentFormat | Renderer | What it does |
|---------------|----------|-------------|
| `markdown` | MarkdownRendererComponent | Renders formatted text, images, links |
| `sophia`, `sophia-quiz-json` | SophiaRendererComponent | Loads `<sophia-question>` web component, emits Recognition events |
| `perseus`, `perseus-quiz-json` | PerseusRendererComponent | Legacy Khan Academy quiz format |
| `html5-app` | IframeRendererComponent | Fetches ZIP from `/apps/{appId}/{entryPoint}`, serves in iframe |
| `gherkin` | GherkinRendererComponent | Renders BDD scenarios |
| `html`, `text` | TextRendererComponent | Simple display |
| `epr-composite` | *not rendered directly* | Parsed by `parsePathView()` into sections/steps/chapters |

### Path navigation flow

```
/lamad/path/{pathId}/step/{stepIndex}
    → PathService.getPath(pathId) → parsePathView() → LearningPath with sections/steps
    → PathService.getStep(pathId, stepIndex) → loads ContentNode for step.resourceId
    → Content viewer renders the step's content via appropriate renderer
```

## Full Data Flow Diagram

```
genesis/docs/*.md
    ↓ elohim-import CLI
genesis/data/lamad/content/*.json
    ↓ seed-sqlite.ts transformContent()
CreateContentInput { contentBody, metadata: {} }
    ↓ POST /db/content/bulk
Doorway (pass-through proxy)
    ↓
elohim-storage HTTP handler
    ↓ serialize_json_opt(metadata → string)
SQLite: content.content_body, content.metadata_json
    ↓ GET /db/content/{id}
elohim-storage response handler
    ↓ parse_json_opt(string → JsonVal)
ContentView { contentBody, metadata: {} }
    ↓ Doorway (pass-through)
Angular ContentService.getContent()
    ↓ transformContent() + resolveBlobUrl()
ContentNode { content, metadata, thumbnailUrl }
    ↓ RendererRegistry.getRenderer()
MarkdownRenderer / SophiaRenderer / IframeRenderer
    ↓
User sees content
```

## Key Files

| Stage | File | Key Lines |
|-------|------|-----------|
| Content definition | `genesis/data/lamad/content/*.json` | |
| Path definition | `genesis/data/lamad/paths/*.json` | |
| Seeder main | `genesis/seeder/src/seed-sqlite.ts` | 853-1146 |
| Content transform | `genesis/seeder/src/seed-sqlite.ts` | 578-613 |
| Path transform | `genesis/seeder/src/seed-sqlite.ts` | 769-798 |
| Schema-generated type | `genesis/seeder/src/generated/create-content-input.ts` | 77-125 |
| Storage HTTP handler | `elohim/elohim-storage/src/http.rs` | 1947-2056 |
| API boundary type | `elohim/elohim-storage/src/views.rs` | 993-1045 |
| DB bulk create | `elohim/elohim-storage/src/db/content_diesel.rs` | 318-390 |
| SQLite schema | `elohim/elohim-storage/migrations/2026-01-08-000000_initial/up.sql` | 37-61 |
| Response type | `elohim/elohim-storage/src/views.rs` | 131-177 |
| Doorway DB proxy | `doorway/doorway-service/src/routes/db.rs` | 35-181 |
| Doorway cache | `doorway/doorway-service/src/routes/api.rs` | 184-257 |
| Connection strategy | `app/elohim-library/.../doorway-connection-strategy.ts` | 226-252 |
| Content service | `app/elohim-app/src/app/elohim/services/content.service.ts` | 305-348, 652-675 |
| Blob URL resolution | `app/elohim-app/src/app/elohim/services/content.service.ts` | 771-809 |
| Content viewer | `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts` | 298-368 |
| Renderer registry | `app/elohim-app/src/app/lamad/renderers/renderer-registry.service.ts` | |
| Path model parsing | `app/elohim-app/src/app/lamad/models/learning-path.model.ts` | 589-640 |
