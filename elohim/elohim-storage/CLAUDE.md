# Elohim Storage - API Boundary Architecture

This crate is the **single source of truth** for the HTTP API that serves TypeScript clients.
All data transformation happens here - TypeScript receives clean, ready-to-use objects.

## Core Principle

snake_case stays inside the Rust boundary; all transformations (JSON parsing, boolean coercion, camelCase conversion) happen in `views.rs`, because TypeScript clients should receive ready-to-use camelCase objects without parsing.

## Unified API for All Clients

A single HTTP surface (`http.rs` + `views.rs`) serves both deployment modes:

- **Browser/Doorway**: `Browser → Doorway → elohim-storage`. Doorway proxies `/db/*`; its projection cache at `/api/v1/cache/*` accelerates reads.
- **Tauri/Direct**: `Tauri App → elohim-storage` on `localhost:8090`. No FFI, no SQLite bindings — same HTTP routes as the proxied path.

## Layer Stack

`HTTP (camelCase JSON) ↔ http.rs (handlers) ↔ views.rs (View/InputView transforms + ts-rs export) ↔ db/*.rs (Diesel, snake_case, String JSON) ↔ SQLite (TEXT for JSON, INTEGER 0/1 for booleans)`. The `db/*` layer is never exposed to HTTP directly.

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
2. **elohim/elohim-views/src/<domain>.rs** - Add View type with `#[derive(TS)]` + ts-rs export attribute (camelCase, Value fields)
3. **elohim/elohim-views/src/inputs.rs** - Add InputView type with `Into<DbInput>` (camelCase, Value fields)
4. **elohim/elohim-storage/src/http.rs** - Add routes using InputView/View types
5. **Regenerate TS** - Run `cargo test export_bindings` from `elohim/elohim-views`

## Schema Contract (view validation)

View types must match their JSON Schema in `../sdk/schemas/v1/views/`.
The `elohim/elohim-storage/tests/schema_contract.rs` integration test validates this at `cargo test` time.

When modifying a View struct:
1. Update the schema first (`elohim/sdk/schemas/v1/views/{name}.schema.json`)
2. Update the Rust struct to match
3. Run `cargo test --test schema_contract` to verify
4. Run `pnpm run schema:codegen:ts` to regenerate TypeScript

---

## Anti-Patterns

- **Exposing DB types to HTTP** — `fn get_content() -> Json<Content>` leaks snake_case; use `Json<ContentView>` instead.
- **JSON parsing in TypeScript** — `JSON.parse(response.metadataJson)` is wrong; Rust already parsed it to `response.metadata`.
- **Transformation functions in TypeScript** — `toWireCreateContent(input)` is wrong; pass camelCase objects through directly, since `views.rs` is the conversion site.

---

## Observation Layer Projections

Per `genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md`, the following existing operational SQL tables are observation projections:

- `peer_blob_inventory` — projects `infrastructure:blob-served` and `infrastructure:blob-hosted` observations. The legacy gossipsub topic `elohim/inventory/blob` is the cursor-announcement stream for both kinds.
- `system_metrics` — projects `infrastructure:system-sample` observations (per-node only per `project_node_metrics_vs_hub_aggregation_boundary`).
- `projection_events` — stays as is (already correctly operational; observation primitive is separate).

The Observation primitive itself lives in the `observations` table (Stage 3.1 migration). Diversity aggregations live in the `observation_diversity_summary` view (Stage 3.2). New observation kinds are declared in pillar manifests under `observation_kinds`.

The Track 2 substrate plane for observations is `IROH_OBSERVATION_ALPN` / libp2p `OBSERVATION_LOG_PROTOCOL_ID` with cursor announcements gossiped via `elohim/observations/<kind_namespace>` topics. See spec §5 for the dataflow.

## Design Vocabulary

Storage and distribution language — `quilt` (RS-encoded distribution of a content unit), `pantry` (peer-tended container), `stock`/`draw` (deposit/retrieve verbs), `shard`, `RS(N,K)` — is defined in `genesis/graphos/vocabulary.md`. Wire-level identifiers (`/blob/{hash}`, `BlobStore`, `sha256-{hex}`) keep their existing names. New design discussion, signal/event names, and any fresh identifiers should use the new vocabulary. Legacy `/store/{hash}` paths were retired 2026-04-30 — the canonical HTTP path is `/blob/{hash}`.

## File Reference

| File | Purpose |
|------|---------|
| `elohim/elohim-views/src/*.rs` | **Wire-shape View types** — ts-rs-anchored, per-domain modules |
| `elohim/elohim-storage/src/views.rs` | Re-export shim + Wire→View `From` impls that touch DB types |
| `elohim/elohim-storage/src/http.rs` | HTTP routes — uses View types via `use elohim_views::...` |
| `elohim/elohim-storage/src/db/models.rs` | Diesel models — internal snake_case |
| `elohim/elohim-storage/src/db/*_diesel.rs` | CRUD operations — internal only |
| `elohim/elohim-storage/src/db/mod.rs` | DB module coordination |

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
