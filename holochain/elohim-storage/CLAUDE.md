# Elohim Storage - API Boundary Architecture

This crate is the **single source of truth** for the HTTP API that serves TypeScript clients.
All data transformation happens here - TypeScript receives clean, ready-to-use objects.

## Core Principle

**snake_case should NEVER leave the Rust boundary.**

All transformations (JSON parsing, boolean coercion, camelCase conversion) happen in `views.rs`.
TypeScript clients receive camelCase objects with properly typed fields - no parsing required.

---

## Unified API for All Clients

This HTTP API serves **both** deployment modes through a single codebase:

```
                    ┌─────────────────────────────────────┐
                    │         elohim-storage              │
                    │   (http.rs / views.rs unified API)  │
                    │                                     │
                    │  /db/content, /session, /store/...  │
                    │         camelCase boundary          │
                    └─────────────────────────────────────┘
                                    ▲
                    ┌───────────────┴───────────────┐
                    │                               │
            ┌───────┴───────┐             ┌────────┴────────┐
            │   Doorway     │             │  Tauri App      │
            │  (proxy at    │             │ (direct HTTP    │
            │  doorway.host)│             │  localhost:8090)│
            └───────────────┘             └─────────────────┘
```

### Browser/Doorway Mode
- Request path: `Browser → Doorway → elohim-storage`
- Doorway proxies `/db/*` requests to elohim-storage
- Also has projection cache at `/api/v1/cache/*` for fast reads

### Tauri/Direct Mode
- Request path: `Tauri App → elohim-storage (localhost:8090)`
- Same HTTP endpoints, just different host
- No proxy, direct connection to local sidecar

**Key Insight**: Tauri does NOT use Rust FFI or direct SQLite bindings. It makes standard HTTP fetch calls to the same `http.rs` endpoints that doorway proxies to. This ensures a single API boundary with consistent behavior.

---

## Architecture Layers

```
HTTP Request (camelCase JSON)
       ↓
┌─────────────────────────────────────────────────┐
│  http.rs - Route handlers                       │
│  - Deserialize → InputView types                │
│  - Convert → DB Input types                     │
│  - Call services                                │
│  - Convert → View types for response            │
└─────────────────────────────────────────────────┘
       ↓                    ↑
┌─────────────────────────────────────────────────┐
│  views.rs - API BOUNDARY (transformation layer) │
│  - InputView: camelCase → snake_case + String   │
│  - View: snake_case + String → camelCase + Value│
│  - ts-rs exports TypeScript types               │
└─────────────────────────────────────────────────┘
       ↓                    ↑
┌─────────────────────────────────────────────────┐
│  db/*.rs - Database operations                  │
│  - Internal types (snake_case, String JSON)     │
│  - Diesel ORM queries                           │
│  - NEVER exposed to HTTP directly               │
└─────────────────────────────────────────────────┘
       ↓                    ↑
┌─────────────────────────────────────────────────┐
│  SQLite - Storage                               │
│  - TEXT for JSON fields                         │
│  - INTEGER for booleans (0/1)                   │
└─────────────────────────────────────────────────┘
```

---

## views.rs Patterns

### Output Views (Response Types)

Transform DB models to camelCase with parsed JSON:

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentView {
    pub id: String,
    pub content_type: String,           // camelCase via serde
    pub metadata: Option<Value>,        // PARSED from metadata_json
    pub is_active: bool,                // COERCED from i32
}

impl From<Content> for ContentView {
    fn from(c: Content) -> Self {
        Self {
            id: c.id,
            content_type: c.content_type,
            metadata: parse_json_opt(&c.metadata_json),  // String → Value
            is_active: c.is_active == 1,                 // i32 → bool
        }
    }
}
```

### Input Views (Request Types)

Accept camelCase with Value, convert to snake_case with String:

```rust
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateContentInputView {
    pub id: String,
    pub content_type: Option<String>,
    pub metadata: Option<Value>,        // PARSED Value from client
}

impl From<CreateContentInputView> for CreateContentInput {
    fn from(v: CreateContentInputView) -> Self {
        Self {
            id: v.id,
            content_type: v.content_type.unwrap_or_else(|| "concept".to_string()),
            metadata_json: serialize_json_opt(&v.metadata),  // Value → String
        }
    }
}
```

---

## Key Transformations

| DB Layer | View Layer | TypeScript |
|----------|------------|------------|
| `metadata_json: Option<String>` | `metadata: Option<Value>` | `metadata?: JsonValue` |
| `is_active: i32` | `isActive: bool` | `isActive: boolean` |
| `content_type: String` | `contentType: String` | `contentType: string` |

---

## Adding New Entities

Follow this workflow:

1. **db/models.rs** - Add Diesel model (snake_case, String JSON fields)
2. **views.rs** - Add View type with `From<Model>` (camelCase, Value fields)
3. **views.rs** - Add InputView type with `Into<DbInput>` (camelCase, Value fields)
4. **http.rs** - Add routes using InputView/View types
5. **Regenerate TS** - Run `cargo test export_bindings`

---

## Anti-Patterns (DO NOT DO)

### Wrong: Exposing DB types to HTTP

```rust
// BAD - exposes snake_case to clients
fn get_content() -> Json<Content> { ... }

// GOOD - uses View type
fn get_content() -> Json<ContentView> { ... }
```

### Wrong: JSON parsing in TypeScript

```typescript
// BAD - parsing in TypeScript
const metadata = JSON.parse(response.metadataJson);

// GOOD - already parsed by Rust
const metadata = response.metadata;  // Ready to use
```

### Wrong: Transformation functions in TypeScript

```typescript
// BAD - toWire/fromWire functions
const wire = toWireCreateContent(input);

// GOOD - direct object passing
const response = await api.createContent(input);  // camelCase in, camelCase out
```

---

## File Reference

| File | Purpose |
|------|---------|
| `src/views.rs` | API boundary - all View/InputView types |
| `src/http.rs` | HTTP routes - uses View types |
| `src/db/models.rs` | Diesel models - internal snake_case |
| `src/db/*_diesel.rs` | CRUD operations - internal only |
| `src/db/mod.rs` | DB module coordination |

---

## Query Parameter Convention

Query structs also use camelCase for URL parameters:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentQuery {
    pub content_type: Option<String>,   // ?contentType=concept
    pub limit: Option<i64>,             // ?limit=100
}
```

TypeScript sends: `?contentType=concept&limit=100`

---

## Testing the Boundary

```bash
# Verify camelCase response
curl http://localhost:8080/db/content/test-id | jq

# Should return:
# {
#   "id": "test-id",
#   "contentType": "concept",
#   "metadata": { ... },
#   "isActive": true
# }
```
