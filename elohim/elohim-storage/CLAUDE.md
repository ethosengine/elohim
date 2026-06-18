---
id: elohim-storage-gospel
cites:
  - tiered-quilt-stewardship-design | the protocol tiered storage substrate design (cold/warm/hot planes, RS sharding, reach enforcement) this crate implements as the operational data plane | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - storage-dual-plane-design-arc | the April 2026 design-arc lineage (dual-plane bet, paths not taken, reach-vocabulary ghost) distilled when the P2P-ARCHITECTURE/EDGE-ARCHITECTURE/REACH island retired | sha256:2315c84345a2ef3c | path: genesis/docs/content/elohim-protocol/history/2026-06-11-storage-dual-plane-design-arc.md
---

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

Per `genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md`, the following existing operational SQL tables are observation projections:

- `peer_blob_inventory` — projects `infrastructure:blob-served` and `infrastructure:blob-hosted` observations. The legacy gossipsub topic `elohim/inventory/blob` is the cursor-announcement stream for both kinds.
- `system_metrics` — projects `infrastructure:system-sample` observations (per-node only per `project_node_metrics_vs_hub_aggregation_boundary`).
- `projection_events` — stays as is (already correctly operational; observation primitive is separate).

The Observation primitive itself lives in the `observations` table (Stage 3.1 migration). Diversity aggregations live in the `observation_diversity_summary` view (Stage 3.2). New observation kinds are declared in pillar manifests under `observation_kinds`.

The Track 2 substrate plane for observations is `IROH_OBSERVATION_ALPN` / libp2p `OBSERVATION_LOG_PROTOCOL_ID` with cursor announcements gossiped via `elohim/observations/<kind_namespace>` topics. See spec §5 for the dataflow.

## Design Vocabulary

Storage and distribution language — `quilt` (RS-encoded distribution of a content unit), `pantry` (peer-tended container), `stock`/`draw` (deposit/retrieve verbs), `shard`, `RS(N,K)` — is defined in `genesis/graphos/vocabulary.md`. Wire-level identifiers (`/blob/{hash}`, `BlobStore`, `sha256-{hex}`) keep their existing names. New design discussion, signal/event names, and any fresh identifiers should use the new vocabulary. Legacy `/store/{hash}` paths were retired 2026-04-30 — the canonical HTTP path is `/blob/{hash}`.

**Addressing canon (CID-first).** The canonical content/blob *address* is a CIDv1 — `bafyrei…` (dag-cbor codec) for EPR atoms / DAG-CBOR content and content-set fingerprints, `bafkrei…` (raw codec, `Cid::new_v1(0x55, Sha2_256(bytes))` — see `src/epr_codec.rs`) for blob bytes. The bare `sha256-<hex>` blob marker still in use on the live `/blob/<hash>` path is the **legacy** form: sha2-256 is only the multihash *inside* a CID, never a standalone address, and a `cid` field must never hold a `sha256-<hex>`. Migrating the blob plane (`/blob`, `BlobStore`, inventory-gossip wire, seeder) from bare-hash to wrapping-CID is a named **downstream arc** — design new surfaces CID-first; describe current behavior accurately. Bare sha stays only for *non-addressing* uses (dedup keys, byte-equality verification, `cite-gen` `sha256:` cite fingerprints). Rule: `.claude/skills/p2p-design-gate` Step 2 "Canonical address forms."

## Identity & Transport-Identity Coherence

A node/agent has three distinct identity namespaces. They are NOT interchangeable:

| Namespace | Example | Home column |
|-----------|---------|-------------|
| Holochain agent key (`agent_cid`) | `uhCAk…` | `humans.agent_pub_key`; `shard_locations.peer_id` (⚠ misnamed — stores `agent_cid`, NOT a libp2p id: verified `seed_shard_manifest.rs:55-58` "a libp2p PeerId will NOT join", `peer_selection.rs:253-255`); `peer_statuses.peer_id`; `rea_commitments.provider` (seeder path) |
| libp2p transport id | `12D3Koo…` | `peer_identity_bindings.peer_id` (handshake-`source` rows); the libp2p swarm |
| iroh `NodeId` | iroh-format | `peer_transport_manifest.iroh_node_id` |

**Rule: never join or match raw identity strings across namespaces.** Raw-string equality between `agent_cid` and a transport peer id silently empties every join — this caused the all-zeros resilience card in `services/household_resilience.rs` (joins at lines 74, 172–174, 447–449).

### Canonical resolver substrate

- **`peer_transport_manifest`** (`src/p2p_iroh/peer_map.rs`) — the Category C operational projection that maps `agent_cid ↔ libp2p_peer_id ↔ iroh_node_id`; exposes `lookup_by_iroh_node_id` and `select_transport`.
- **`AgentPeerBinding`** — a notarized DHT integrity entry (listed in `is_integrity_kind` at `src/write_through.rs:211`); emitted as `AppSignal::AgentPeerBindingCreated` (`src/signals.rs:1194`); projected by `ReconcileController::on_agent_peer_binding` (`src/reconcile/controller.rs:549`) into the `peer_identity_bindings` table.
- **`src/node_transport.rs`** — the self-identity seam: resolves the node's own `self_cid` (which namespace it returns depends on transport mode — libp2p returns the peer id string; iroh returns the iroh NodeId). Mismatches between `self_cid` and seeder-written `provider` values are the primary fragmentation source.
- **`src/p2p/identity_handshake.rs`** — `synthesise_dht_anchor_hash(peer_id, agent_cid)` (line 336) is a fallback anchor when no DHT-truth row exists yet; it is not a substitute resolver.

Pick `agent_cid` as the canonical join key. **Most identities you will join ARE already `agent_cid`** — `shard_locations.peer_id` and (seeder-written) `rea_commitments.provider` both hold `uhCAk`; the live cause of an empty join is usually a NULL `humans.agent_pub_key`, fixed by **populating it**, not by a resolver. ⚠ A general transport-id→`agent_cid` resolver is specced (`genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md`) but is **NOT built and is blocked**: no edge node emits a real `AgentPeerBinding` (projected rows are split-brained placeholders), and the binding is **self-asserted/unsigned today** (`STAGE1_SIGNATURE_SENTINEL`; agent and libp2p keys are uncrossed) — do NOT consume bindings for economic attribution until a cross-signed control proof lands (open security item).

---

## P2P Data Plane & Reach (concern routing)

- **Data-plane architecture truth** is the tiered-quilt design (cited above): three truth layers, pantry-temperature classes, custody commitments. The April 2026 design-arc lineage (dual-plane bet, paths not taken) is the `storage-dual-plane-design-arc` history record (cited above) — read the canon, not retired drafts.
- **Reach vocabulary**: DNA-notarized 8 values in `elohim/sdk/schemas/v1/enums/reach.schema.json` (matched by `elohim/epr/src/reach.rs`); the standing-policy family is `src/services/epr_kind.rs`. Multi-vocabulary drift is a known OPEN item — `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md` + the resilience README reconciliation (roadmap item 13); do not canonize any single vocabulary as resolved.
- **Reach enforcement** is author-side earning + receiver-side pre-authorization (`src/p2p/reach_authorization.rs`), not delivery-side filtering. The HTTP-path enforcement gap is tracked in `genesis/data/timeline/backlog/http-reach-enforcement-gap.md`; unconsumed sovereignty/cluster scaffolding is recorded in `genesis/data/timeline/backlog/storage-island-harvest-residue.md`.

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
