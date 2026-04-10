# Peer Status Schema Contract — Implementation Plan

**Date:** 2026-04-09
**Design doc:** `genesis/plans/2026-04-09-peer-status-schema-contract-design.md`
**Branch:** `dev` (direct, no feature branch)
**Commit strategy:** One commit at sprint end — no per-phase commits

## Scope

Phases 0–8 of the design doc. Tier 3 plumbing (Phases 7–10 in the design doc: RTT ring buffer, lastSeen map, remoteNatStatus dial-back, bandwidth sinks) is **deferred to a follow-up sprint**. The design doc anticipates this: "phases 7–11 can defer to a follow-up sprint without structural change."

## Key Discovery: codegen-rs Does NOT Generate Structs

`codegen-rs.mjs` only generates Rust enum CONSTANTS (`&[&str]` arrays from `enums/*.schema.json`). It does NOT generate Rust structs from view schemas. This plan keeps Rust structs hand-written but adds a **validation harness** (Rust integration test) that serializes each struct and validates the JSON against the schema. The harness catches struct↔schema drift at `cargo test` time. Schema→Rust struct codegen is a future improvement.

## Consumer Map (snake_case → camelCase migration)

These consumers read `/p2p/status` and will break when P2PStatusInfo switches to camelCase:

| Consumer | File | Fields Read |
|----------|------|-------------|
| Doorway federation | `doorway/doorway-service/src/routes/federation.rs:297–311` | `peer_id`, `listen_addresses`, `nat_status`, `relay_mode` |
| Connection indicator | `app/elohim-app/src/app/imagodei/components/connection-indicator/connection-indicator.component.ts:206–227` | `connected_peers` |
| simulate.sh | `steward/node/simulation/simulate.sh:162,208` | `connected_peers` (grep pattern) |
| Orchestrator Jenkinsfile | `genesis/orchestrator/Jenkinsfile:149–153` | `connected_peers`, `peer_id` (via doorway health → cascades) |
| Genesis Jenkinsfile | `genesis/Jenkinsfile:611–614` | reads `/p2p/status` (check fields parsed) |
| Doorway health embed | `doorway/doorway-service/src/server/http.rs:612` | embeds full `/p2p/status` response into health |

---

## Phase 0: Conventions & Audit

### Task 0.1: Write CONVENTIONS.md

**File:** `elohim/sdk/schemas/v1/views/CONVENTIONS.md`

Create the conventions reference for all view schemas. Content derived from existing `content-view.schema.json` patterns + design doc rules.

```markdown
# View Schema Conventions

View schemas define the JSON wire format for HTTP API responses. They are the
**single source of truth** for the shape of data that crosses the Rust→TypeScript
boundary via HTTP.

## Rules

### 1. camelCase field names
All properties use camelCase. The Rust struct uses `#[serde(rename_all = "camelCase")]`.

### 2. Source of truth declaration (REQUIRED)
The top-level `description` field MUST declare the entity's source of truth
and its P2P design gate category (A/A2/B/B2/C). Examples:

- Category A: `"Source of truth: DHT (Notarized, Category A)."`
- Category C: `"Source of truth: libp2p Swarm state (Operational, Category C). Reconstructed per request. Not persisted."`

The validation harness enforces this: a schema without "Source of truth:" in
the description fails the contract test.

### 3. additionalProperties: false
Every view schema MUST set `additionalProperties: false`. This prevents
undeclared fields from leaking through and makes the contract tight.

### 4. required array
Every non-nullable field MUST appear in the `required` array.

### 5. Nullable fields
Use JSON Schema nullable pattern: `{ "type": ["string", "null"] }`.
The `required` array determines whether the field must be present;
the type determines whether its value can be null.

### 6. $id format
Use EPR-style IDs: `epr:schema:view:{name}` (e.g., `epr:schema:view:p2p-status`).

### 7. Enum references
Use `$ref` to reference enum schemas in `../enums/`. Never inline enum values
in view schemas — the enum schema is the single source of truth.

### 8. Integer types
Use `"type": "integer"` for counts and indices. For large values (e.g.,
uptimeSeconds as u64), use `"type": "string"` with a `"pattern"` constraint
and document the bigint-as-string convention.

### 9. Nested objects
Use `$ref` to reference object schemas for nested types (e.g., replication
status, drain status). Define these as separate schemas in `views/` or
`objects/` as appropriate.

### 10. File naming
`{entity-name}.schema.json` in kebab-case matching the `$id` suffix.
```

### Task 0.2: Audit economic-event-view.schema.json

**File:** `elohim/sdk/schemas/v1/views/economic-event-view.schema.json`

Fix two convention violations found during research:

**Fix 1:** Add missing `additionalProperties: false` (content-view has it, economic-event-view doesn't).

**Fix 2:** The schema has no `required` for non-nullable fields beyond `["id", "action", "provider", "receiver", "state", "createdAt"]`. This is correct for the current usage where most fields are optional, but verify it matches the Rust struct's serialization.

```json
// Add at the end of the schema object, before the closing }:
"additionalProperties": false
```

### Task 0.3: Verify existing schemas pass conventions

After writing CONVENTIONS.md, manually verify both existing view schemas against the 10 rules. Document any violations found as comments in the plan file or fix them inline.

- `content-view.schema.json`: Has additionalProperties:false, has required, has Source of truth in description. **PASS.**
- `economic-event-view.schema.json`: Missing additionalProperties:false (fixed in 0.2). Has Source of truth in description. **PASS after fix.**

---

## Phase 1: Write P2P View Schemas

### Task 1.1: Write drain-status-view.schema.json

**File:** `elohim/sdk/schemas/v1/views/drain-status-view.schema.json`

Separated from P2PStatusView because it's referenced via `$ref` and may be used independently.

```json
{
  "$id": "epr:schema:view:drain-status",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "DrainStatusView",
  "description": "Drain queue state for DHT publication. Source of truth: SQLite aggregate query over p2p_published_at column (Operational, Category C).",
  "type": "object",
  "required": ["total", "published", "pending"],
  "properties": {
    "total": {
      "type": "integer",
      "description": "Total rows in the local content projection (scoped to lamad app)"
    },
    "published": {
      "type": "integer",
      "description": "Rows successfully published to libp2p Kad DHT"
    },
    "pending": {
      "type": "integer",
      "description": "Rows not yet drained. 0 and stable = caught up"
    }
  },
  "additionalProperties": false
}
```

### Task 1.2: Write replication-status-view.schema.json

**File:** `elohim/sdk/schemas/v1/views/replication-status-view.schema.json`

```json
{
  "$id": "epr:schema:view:replication-status",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ReplicationStatusView",
  "description": "Identity-driven content replication progress. Source of truth: in-memory ReplicationInner struct (Operational, Category C). Rebuilt from discovery on every process start.",
  "type": "object",
  "required": ["pending", "completed", "failed", "caughtUp"],
  "properties": {
    "pending": {
      "type": "integer",
      "description": "Content IDs discovered but not yet fetched"
    },
    "completed": {
      "type": "integer",
      "description": "Content IDs successfully replicated"
    },
    "failed": {
      "type": "integer",
      "description": "Content IDs that failed fetch (will retry)"
    },
    "caughtUp": {
      "type": "boolean",
      "description": "True when all discovered content has been fetched or failed with max retries"
    }
  },
  "additionalProperties": false
}
```

### Task 1.3: Write nat-status enum schema

**File:** `elohim/sdk/schemas/v1/enums/nat-status.schema.json`

Formalizes the currently-stringly-typed `nat_status` field.

```json
{
  "$id": "epr:schema:enum:nat-status",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "NatStatus",
  "description": "NAT traversal status as detected by libp2p autonat protocol",
  "type": "string",
  "enum": ["unknown", "public", "private"]
}
```

Note: No `_dna` metadata — this enum is NOT DNA-notarized (Category C operational). The codegen-rs script skips schemas without `_dna`, so this won't generate Rust constants and doesn't need to.

### Task 1.4: Write relay-mode enum schema

**File:** `elohim/sdk/schemas/v1/enums/relay-mode.schema.json`

```json
{
  "$id": "epr:schema:enum:relay-mode",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RelayMode",
  "description": "Relay operating mode for libp2p circuit relay",
  "type": "string",
  "enum": ["disabled", "client", "server", "both"]
}
```

### Task 1.5: Write p2p-status-view.schema.json

**File:** `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json`

The main schema. References the sub-schemas via `$ref`.

```json
{
  "$id": "epr:schema:view:p2p-status",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "P2PStatusView",
  "description": "P2P node status for observability. Source of truth: libp2p Swarm state + in-memory replication tracker + SQLite drain query (Operational, Category C). Reconstructed per request. Not persisted.",
  "type": "object",
  "required": [
    "peerId",
    "listenAddresses",
    "connectedPeers",
    "bootstrapNodes",
    "syncDocuments",
    "natStatus",
    "relayReservations",
    "announceAddresses",
    "relayMode",
    "replication"
  ],
  "properties": {
    "peerId": {
      "type": "string",
      "description": "libp2p PeerId (base58 encoded)"
    },
    "listenAddresses": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Multiaddrs this node is listening on"
    },
    "connectedPeers": {
      "type": "integer",
      "description": "Number of currently connected peers"
    },
    "bootstrapNodes": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Configured bootstrap node multiaddrs"
    },
    "syncDocuments": {
      "type": "integer",
      "description": "Number of Automerge sync documents"
    },
    "natStatus": {
      "$ref": "../enums/nat-status.schema.json",
      "description": "NAT status detected by autonat"
    },
    "relayReservations": {
      "type": "integer",
      "description": "Number of active relay reservations"
    },
    "announceAddresses": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Addresses announced to the network"
    },
    "relayMode": {
      "$ref": "../enums/relay-mode.schema.json",
      "description": "Relay mode this node is running in"
    },
    "replication": {
      "$ref": "replication-status-view.schema.json",
      "description": "Identity-driven content replication progress"
    },
    "drain": {
      "oneOf": [
        { "$ref": "drain-status-view.schema.json" },
        { "type": "null" }
      ],
      "description": "Drain queue state. null when DB pool or query unavailable — treat as 'data not available', NOT 'caught up'"
    }
  },
  "additionalProperties": false
}
```

### Task 1.6: Write peer-info-view.schema.json

**File:** `elohim/sdk/schemas/v1/views/peer-info-view.schema.json`

Per-peer detail for the new `/p2p/peers` endpoint. Tier 1+2 fields only; Tier 3 nullable fields present but documented as "populated in follow-up sprint."

```json
{
  "$id": "epr:schema:view:peer-info",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "PeerInfoView",
  "description": "Per-peer detail from libp2p Swarm state. Source of truth: libp2p identify protocol + per-peer runtime tracking (Operational, Category C). All ephemeral.",
  "type": "object",
  "required": ["peerId", "multiaddrs", "protocols", "agentVersion", "direction"],
  "properties": {
    "peerId": {
      "type": "string",
      "description": "libp2p PeerId (base58 encoded)"
    },
    "multiaddrs": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Known multiaddrs for this peer (from identify)"
    },
    "protocols": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Protocols supported by this peer (from identify)"
    },
    "agentVersion": {
      "type": "string",
      "description": "Agent version string from identify protocol"
    },
    "direction": {
      "type": "string",
      "enum": ["inbound", "outbound"],
      "description": "Connection direction — who initiated"
    },
    "rttMs": {
      "type": ["number", "null"],
      "description": "Round-trip time in milliseconds. Tier 3 — null until follow-up sprint implements RTT ring buffer"
    },
    "lastSeenMs": {
      "type": ["integer", "null"],
      "description": "Unix epoch millis of last activity. Tier 3 — null until follow-up sprint implements lastSeen tracking"
    },
    "remoteNatStatus": {
      "oneOf": [
        { "$ref": "../enums/nat-status.schema.json" },
        { "type": "null" }
      ],
      "description": "Remote peer's NAT status. Tier 3 — null until follow-up sprint implements dial-back results"
    },
    "bandwidthIn": {
      "type": ["integer", "null"],
      "description": "Bytes received from this peer since connection. Tier 3 — null until follow-up sprint implements BandwidthSinks"
    },
    "bandwidthOut": {
      "type": ["integer", "null"],
      "description": "Bytes sent to this peer since connection. Tier 3 — null until follow-up sprint implements BandwidthSinks"
    }
  },
  "additionalProperties": false
}
```

### Task 1.7: Write peer-list-view.schema.json

**File:** `elohim/sdk/schemas/v1/views/peer-list-view.schema.json`

Pagination envelope for `/p2p/peers`.

```json
{
  "$id": "epr:schema:view:peer-list",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "PeerListView",
  "description": "Paginated list of connected peers. Source of truth: libp2p Swarm state (Operational, Category C).",
  "type": "object",
  "required": ["peers", "total"],
  "properties": {
    "peers": {
      "type": "array",
      "items": { "$ref": "peer-info-view.schema.json" },
      "description": "Connected peers with detail"
    },
    "total": {
      "type": "integer",
      "description": "Total connected peer count"
    }
  },
  "additionalProperties": false
}
```

---

## Phase 2: Validation Harness

### Task 2.1: Add jsonschema crate to elohim-storage dev-dependencies

**File:** `elohim/elohim-storage/Cargo.toml`

Add under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"
jsonschema = "0.29"
```

### Task 2.2: Write schema contract test

**File:** `elohim/elohim-storage/tests/schema_contract.rs`

Integration test that serializes Rust structs and validates against schema files. Initially this test will FAIL because P2PStatusInfo serializes snake_case but the schema requires camelCase. Phase 3 fixes this.

```rust
//! Schema contract tests — validates that Rust serialization matches
//! the JSON Schema source of truth in elohim/sdk/schemas/v1/views/.
//!
//! These tests catch drift between Rust struct changes and the schema
//! contract. If a field is renamed, added, or removed in Rust without
//! updating the schema (or vice versa), these tests fail.

use elohim_storage::p2p::replication::ReplicationStatus;
use elohim_storage::{DrainStatusInfo, P2PStatusInfo};
use jsonschema::JSONSchema;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Resolve a schema file relative to the repo root.
fn schema_path(relative: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // elohim/elohim-storage/ → elohim/sdk/schemas/v1/
    manifest_dir.join("../sdk/schemas/v1").join(relative)
}

/// Load and compile a JSON Schema, resolving $ref within the schema dir.
fn load_schema(relative: &str) -> (Value, JSONSchema) {
    let path = schema_path(relative);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read schema {}: {}", path.display(), e));
    let schema_value: Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse schema {}: {}", path.display(), e));
    let compiled = JSONSchema::compile(&schema_value)
        .unwrap_or_else(|e| panic!("Failed to compile schema {}: {}", path.display(), e));
    (schema_value, compiled)
}

/// Validate a serialized Rust struct against a schema.
fn validate_against_schema(schema_path_str: &str, instance: &Value) {
    let (_schema_value, compiled) = load_schema(schema_path_str);
    let result = compiled.validate(instance);
    if let Err(errors) = result {
        let error_msgs: Vec<String> = errors.map(|e| format!("  - {}", e)).collect();
        panic!(
            "Schema validation failed for {}:\n{}\n\nInstance:\n{}",
            schema_path_str,
            error_msgs.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }
}

/// Convention: every view schema must declare source of truth in description.
fn assert_source_of_truth_declared(schema_value: &Value, schema_name: &str) {
    let desc = schema_value["description"]
        .as_str()
        .unwrap_or_else(|| panic!("Schema {} missing description", schema_name));
    assert!(
        desc.contains("Source of truth:"),
        "Schema {} description must contain 'Source of truth:' — got: {}",
        schema_name,
        desc
    );
}

#[test]
fn p2p_status_view_matches_schema() {
    let status = P2PStatusInfo {
        peer_id: "12D3KooWTest".to_string(),
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
        connected_peers: 3,
        bootstrap_nodes: vec!["/dnsaddr/bootstrap.example.com".to_string()],
        sync_documents: 5,
        nat_status: "public".to_string(),
        relay_reservations: 1,
        announce_addresses: vec!["/ip4/1.2.3.4/tcp/4001".to_string()],
        relay_mode: "client".to_string(),
        replication: ReplicationStatus {
            pending: 2,
            completed: 10,
            failed: 0,
            caught_up: false,
        },
        drain: Some(DrainStatusInfo {
            total: 100,
            published: 95,
            pending: 5,
        }),
    };

    let json = serde_json::to_value(&status).unwrap();
    validate_against_schema("views/p2p-status-view.schema.json", &json);
}

#[test]
fn p2p_status_view_with_null_drain() {
    let status = P2PStatusInfo {
        peer_id: "12D3KooWTest".to_string(),
        listen_addresses: vec![],
        connected_peers: 0,
        bootstrap_nodes: vec![],
        sync_documents: 0,
        nat_status: "unknown".to_string(),
        relay_reservations: 0,
        announce_addresses: vec![],
        relay_mode: "disabled".to_string(),
        replication: ReplicationStatus::default(),
        drain: None,
    };

    let json = serde_json::to_value(&status).unwrap();
    validate_against_schema("views/p2p-status-view.schema.json", &json);
}

#[test]
fn drain_status_view_matches_schema() {
    let drain = DrainStatusInfo {
        total: 100,
        published: 95,
        pending: 5,
    };

    let json = serde_json::to_value(&drain).unwrap();
    validate_against_schema("views/drain-status-view.schema.json", &json);
}

#[test]
fn replication_status_view_matches_schema() {
    let replication = ReplicationStatus {
        pending: 10,
        completed: 50,
        failed: 2,
        caught_up: false,
    };

    let json = serde_json::to_value(&replication).unwrap();
    validate_against_schema("views/replication-status-view.schema.json", &json);
}

#[test]
fn view_schemas_declare_source_of_truth() {
    let view_schemas = [
        "views/p2p-status-view.schema.json",
        "views/drain-status-view.schema.json",
        "views/replication-status-view.schema.json",
        "views/peer-info-view.schema.json",
        "views/peer-list-view.schema.json",
        "views/content-view.schema.json",
        "views/economic-event-view.schema.json",
    ];

    for schema_name in &view_schemas {
        let path = schema_path(schema_name);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", schema_name, e));
        let schema_value: Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", schema_name, e));
        assert_source_of_truth_declared(&schema_value, schema_name);
    }
}
```

**IMPORTANT:** The `p2p_status_view_matches_schema` test will FAIL until Phase 3 switches P2PStatusInfo to camelCase. This is intentional TDD — the schema is the spec, the test enforces it, Phase 3 makes it pass.

### Task 2.3: Verify the harness compiles (but expect test failures)

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract -- --compile-only 2>&1 || true
```

If compilation fails, fix imports. The `P2PStatusInfo`, `DrainStatusInfo`, and `ReplicationStatus` types must be publicly exported. Check `src/lib.rs:122` — `P2PStatusInfo` and `DrainStatusInfo` are already re-exported. `ReplicationStatus` is exported via `p2p::replication::ReplicationStatus`. The test file's `use` statements may need adjustment based on the actual module structure.

---

## Phase 3: Rust Struct Alignment (camelCase Migration)

### Task 3.1: Switch P2PStatusInfo to camelCase

**File:** `elohim/elohim-storage/src/p2p/mod.rs`

Replace the current struct definition (lines 244–277) with camelCase serde. Remove the NOTE comment about snake_case backward compatibility — the schema contract replaces that reasoning.

Change:
```rust
/// P2P node status for observability.
///
/// NOTE: This struct intentionally does NOT use `rename_all = "camelCase"`
/// because the wire format on `/p2p/status` has historically been snake_case
/// ...
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct P2PStatusInfo {
    pub peer_id: String,
    ...
```

To:
```rust
/// P2P node status for observability.
///
/// Wire format governed by: `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json`
/// Schema contract test: `tests/schema_contract.rs::p2p_status_view_matches_schema`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct P2PStatusInfo {
    pub peer_id: String,
    ...
```

Also update `nat_status` field type annotation for schema alignment — the schema says values are `"unknown"`, `"public"`, `"private"` (lowercase). Check the current Debug format output from libp2p's NatStatus and ensure it matches. If libp2p emits `"Public"` (capitalized), add a `.to_lowercase()` in the status construction code.

### Task 3.2: Verify harness passes

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract 2>&1
```

All 5 tests should pass now. If `nat_status` values don't match enum (e.g., `"Unknown"` vs `"unknown"`), fix the status construction in `p2p/mod.rs` (around line 596 and 3182) to normalize to lowercase.

### Task 3.3: Regenerate ts-rs TypeScript types

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1
```

This updates `elohim/sdk/storage-client-ts/src/generated/P2PStatusInfo.ts` to emit camelCase field names.

---

## Phase 4: Consumer Migration (snake_case → camelCase)

### Task 4.1: Migrate doorway federation.rs

**File:** `doorway/doorway-service/src/routes/federation.rs`

In `query_storage_p2p_status()` (line 277+), change all JSON field access from snake_case to camelCase:

```rust
// Before:
let peer_id = body["peer_id"].as_str()...
let multiaddrs: Vec<String> = body["listen_addresses"].as_array()...
let nat_status = body["nat_status"].as_str()...
let relay_mode = body["relay_mode"].as_str()...

// After:
let peer_id = body["peerId"].as_str()...
let multiaddrs: Vec<String> = body["listenAddresses"].as_array()...
let nat_status = body["natStatus"].as_str()...
let relay_mode = body["relayMode"].as_str()...
```

### Task 4.2: Migrate doorway health embed

**File:** `doorway/doorway-service/src/server/http.rs` (around line 612)

Check how the doorway health endpoint embeds P2P status. If it reads the raw JSON response from `/p2p/status` and nests it under a `"p2p"` key, the camelCase change cascades automatically (the raw JSON is already camelCase after Phase 3). No code change needed in that case — just verify.

If it deserializes into a typed struct and re-serializes, update the struct's field access.

### Task 4.3: Migrate doorway main.rs

**File:** `doorway/doorway-service/src/main.rs` (around line 359)

Same pattern as Task 4.2 — check if it reads `/p2p/status` and how it uses the response. Update field names if deserialized.

### Task 4.4: Migrate connection-indicator component

**File:** `app/elohim-app/src/app/imagodei/components/connection-indicator/connection-indicator.component.ts`

Line 206: Change the type annotation and field access:
```typescript
// Before:
.get<{ connected_peers?: number; peer_count?: number }>(`${baseUrl}/p2p/status`)

// After:
.get<{ connectedPeers?: number; peer_count?: number }>(`${baseUrl}/p2p/status`)
```

Line 224: Change field access:
```typescript
// Before:
(resp as { connected_peers?: number }).connected_peers ??

// After:
(resp as { connectedPeers?: number }).connectedPeers ??
```

Note: `peer_count` is from the doorway health endpoint, which may have its own format — keep that fallback path as-is unless the doorway health response also changes.

### Task 4.5: Update connection-indicator tests

**File:** `app/elohim-app/src/app/imagodei/components/connection-indicator/connection-indicator.component.spec.ts`

If any mock responses use `connected_peers`, change to `connectedPeers`.

### Task 4.6: Migrate simulate.sh

**File:** `steward/node/simulation/simulate.sh`

Lines 162 and 208: Change the grep pattern:
```bash
# Before:
peers=$(curl -sf "http://localhost:$port/p2p/status" 2>/dev/null | grep -o '"connected_peers":[0-9]*' | grep -o '[0-9]*' || echo "0")

# After:
peers=$(curl -sf "http://localhost:$port/p2p/status" 2>/dev/null | grep -o '"connectedPeers":[0-9]*' | grep -o '[0-9]*' || echo "0")
```

### Task 4.7: Migrate orchestrator Jenkinsfile

**File:** `genesis/orchestrator/Jenkinsfile`

Lines 149–153: Change Python field access (but note this reads from doorway health endpoint, not /p2p/status directly). If the doorway health endpoint nests the raw P2P JSON:

```python
# Before:
p.get('connected_peers',0)
p.get('peer_id','unknown')

# After:
p.get('connectedPeers',0)
p.get('peerId','unknown')
```

Also update line 153's display string to use the new field name in the echo.

### Task 4.8: Migrate genesis Jenkinsfile

**File:** `genesis/Jenkinsfile`

Lines 611–614: Check what fields the seeder health-check reads from `/p2p/status`. Update any snake_case field access to camelCase. The replication polling likely reads `replication.caughtUp` — this was ALREADY camelCase in ReplicationStatus, so it should be fine. Verify.

---

## Phase 5: TypeScript Codegen & Distribution

### Task 5.1: Add new view schemas to INTERFACE_FILES

**File:** `elohim/sdk/schemas/scripts/codegen-ts.mjs`

Add new entries to the `INTERFACE_FILES` array (line 35):

```javascript
const INTERFACE_FILES = [
  { src: 'inputs/create-content-input.ts', dest: 'create-content-input.ts' },
  { src: 'inputs/create-economic-event-input.ts', dest: 'create-economic-event-input.ts' },
  { src: 'inputs/create-attestation-input.ts', dest: 'create-attestation-input.ts' },
  { src: 'views/content-view.ts', dest: 'content-view.ts' },
  { src: 'views/economic-event-view.ts', dest: 'economic-event-view.ts' },
  // New P2P view schemas
  { src: 'views/p2p-status-view.ts', dest: 'p2p-status-view.ts' },
  { src: 'views/drain-status-view.ts', dest: 'drain-status-view.ts' },
  { src: 'views/replication-status-view.ts', dest: 'replication-status-view.ts' },
  { src: 'views/peer-info-view.ts', dest: 'peer-info-view.ts' },
  { src: 'views/peer-list-view.ts', dest: 'peer-list-view.ts' },
];
```

### Task 5.2: Add $ref resolution for view-to-view references

**File:** `elohim/sdk/schemas/scripts/codegen-ts.mjs`

The existing `loadRefMap()` function (line 47) only loads `$ref` targets from the `enums/` directory. The new P2P schemas use `$ref` across views (e.g., `p2p-status-view` references `drain-status-view`). Extend `loadRefMap()` to also load view schemas:

```javascript
async function loadRefMap(baseDir) {
  const refMap = new Map();

  // Load enum schemas (existing)
  const enumDir = join(baseDir, 'enums');
  // ... existing code ...

  // Load view schemas for cross-view $ref
  const viewDir = join(baseDir, 'views');
  let viewFiles;
  try {
    viewFiles = (await readdir(viewDir)).filter((f) => f.endsWith('.schema.json'));
  } catch {
    viewFiles = [];
  }
  for (const file of viewFiles) {
    const schema = JSON.parse(await readFile(join(viewDir, file), 'utf8'));
    // View-to-view refs use just the filename (same directory)
    refMap.set(file, schema);
    // Also support path from enums/ reference point
    refMap.set(`../views/${file}`, schema);
  }

  return refMap;
}
```

### Task 5.3: Run codegen and verify distribution

```bash
cd /projects/elohim
pnpm run schema:codegen:ts
```

Verify that new files appear in all 3 distribution targets:
- `genesis/seeder/src/generated/p2p-status-view.ts`
- `app/elohim-app/src/app/generated/p2p-status-view.ts`
- `app/elohim-library/projects/elohim-service/src/generated/p2p-status-view.ts`

(And similarly for drain-status-view, replication-status-view, peer-info-view, peer-list-view.)

### Task 5.4: Verify codegen freshness passes

```bash
pnpm run schema:codegen:ts -- --verify
pnpm run schema:codegen:rs -- --verify
```

Both should pass (codegen-rs only processes enums, and we haven't changed any enum schemas that have `_dna` metadata).

---

## Phase 6: Add /p2p/peers Endpoint

### Task 6.1: Add PeerInfoView and PeerListView structs to Rust

**File:** `elohim/elohim-storage/src/p2p/mod.rs`

Add new structs after `P2PStatusInfo`. These match the schemas written in Phase 1:

```rust
/// Per-peer detail from libp2p Swarm state.
///
/// Wire format governed by: `elohim/sdk/schemas/v1/views/peer-info-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerInfoView {
    pub peer_id: String,
    pub multiaddrs: Vec<String>,
    pub protocols: Vec<String>,
    pub agent_version: String,
    pub direction: String,
    /// Tier 3 — populated in follow-up sprint
    pub rtt_ms: Option<f64>,
    /// Tier 3 — populated in follow-up sprint
    #[ts(type = "number | null")]
    pub last_seen_ms: Option<u64>,
    /// Tier 3 — populated in follow-up sprint
    pub remote_nat_status: Option<String>,
    /// Tier 3 — populated in follow-up sprint
    #[ts(type = "number | null")]
    pub bandwidth_in: Option<u64>,
    /// Tier 3 — populated in follow-up sprint
    #[ts(type = "number | null")]
    pub bandwidth_out: Option<u64>,
}

/// Paginated list of connected peers.
///
/// Wire format governed by: `elohim/sdk/schemas/v1/views/peer-list-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerListView {
    pub peers: Vec<PeerInfoView>,
    #[ts(type = "number")]
    pub total: usize,
}
```

### Task 6.2: Add peer enumeration to P2PHandle

**File:** `elohim/elohim-storage/src/p2p/mod.rs`

The P2PHandle needs a method that queries the swarm for connected peer info. This requires either:
(a) A new P2PCommand variant + response from the event loop, or
(b) A shared data structure populated by the event loop

Option (b) is simpler for Tier 1+2 (we already have this pattern with `delivery_peers: Arc<DashMap<...>>`). Add a similar structure for general peer info.

Add a new command to the event loop that returns connected peer info:

```rust
pub enum P2PCommand {
    // ... existing variants ...
    /// List connected peers with identify info
    ListPeers {
        reply: oneshot::Sender<Vec<PeerInfoView>>,
    },
}
```

In P2PHandle, add:
```rust
pub async fn list_peers(&self) -> Vec<PeerInfoView> {
    let (tx, rx) = oneshot::channel();
    let _ = self.command_tx.send(P2PCommand::ListPeers { reply: tx }).await;
    rx.await.unwrap_or_default()
}
```

In the P2P event loop (the `select!` match arm for commands), handle `ListPeers`:
```rust
P2PCommand::ListPeers { reply } => {
    let peers: Vec<PeerInfoView> = self.swarm
        .connected_peers()
        .map(|peer_id| {
            let info = self.swarm.behaviour().identify.info_of(peer_id);
            PeerInfoView {
                peer_id: peer_id.to_string(),
                multiaddrs: info.map(|i| i.listen_addrs.iter().map(|a| a.to_string()).collect()).unwrap_or_default(),
                protocols: info.map(|i| i.protocols.iter().map(|p| p.to_string()).collect()).unwrap_or_default(),
                agent_version: info.map(|i| i.agent_version.clone()).unwrap_or_default(),
                direction: "unknown".to_string(), // TODO: track in connection handler
                rtt_ms: None,        // Tier 3
                last_seen_ms: None,  // Tier 3
                remote_nat_status: None, // Tier 3
                bandwidth_in: None,  // Tier 3
                bandwidth_out: None, // Tier 3
            }
        })
        .collect();
    let _ = reply.send(peers);
}
```

**Note:** The exact API for `self.swarm.behaviour().identify.info_of(peer_id)` depends on the libp2p identify behaviour's API. Read the actual identify behaviour struct to find the correct method. In libp2p 0.54, it's typically `Identify::peer_info(peer_id)` or stored in the identify cache. Adjust based on what's available.

### Task 6.3: Add /p2p/peers route

**File:** `elohim/elohim-storage/src/http.rs`

Add the new route near the existing `/p2p/status` match (line 505):

```rust
(Method::GET, "/p2p/status") => self.handle_p2p_status().await,
(Method::GET, "/p2p/peers") => self.handle_p2p_peers().await,
```

Add the handler:

```rust
async fn handle_p2p_peers(&self) -> Result<Response<Full<Bytes>>, StorageError> {
    if let Some(ref handle) = self.p2p_handle {
        let peers = handle.list_peers().await;
        let total = peers.len();
        let response = PeerListView { peers, total };
        let json = serde_json::to_string(&response).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize peer list: {}", e))
        })?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap())
    } else {
        let empty = PeerListView { peers: vec![], total: 0 };
        let json = serde_json::to_string(&empty).unwrap();
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap())
    }
}
```

### Task 6.4: Add /p2p/peers to storage manifest

**File:** `elohim/elohim-storage/src/http.rs`

Find the `build_manifest()` function. Add the new route so doorway auto-discovers it:

```rust
// In build_manifest(), add:
routes.push(ManifestRoute {
    method: "GET".to_string(),
    path: "/p2p/peers".to_string(),
    description: "List connected P2P peers with detail".to_string(),
});
```

### Task 6.5: Add proxy config for /p2p/peers

**File:** `app/elohim-app/proxy.conf.mjs`

Add `/p2p` to the proxy context list if not already present:

```javascript
context: ['/api', '/db', '/blob', '/apps', '/health', '/p2p', '/epr-head'],
```

### Task 6.6: Add harness tests for PeerInfoView and PeerListView

**File:** `elohim/elohim-storage/tests/schema_contract.rs`

Add tests for the new types:

```rust
use elohim_storage::p2p::{PeerInfoView, PeerListView};

#[test]
fn peer_info_view_matches_schema() {
    let peer = PeerInfoView {
        peer_id: "12D3KooWPeer1".to_string(),
        multiaddrs: vec!["/ip4/192.168.1.1/tcp/4001".to_string()],
        protocols: vec!["/elohim/shard/1.0.0".to_string()],
        agent_version: "elohim-storage/0.1.0".to_string(),
        direction: "outbound".to_string(),
        rtt_ms: None,
        last_seen_ms: None,
        remote_nat_status: None,
        bandwidth_in: None,
        bandwidth_out: None,
    };

    let json = serde_json::to_value(&peer).unwrap();
    validate_against_schema("views/peer-info-view.schema.json", &json);
}

#[test]
fn peer_list_view_matches_schema() {
    let list = PeerListView {
        peers: vec![PeerInfoView {
            peer_id: "12D3KooWPeer1".to_string(),
            multiaddrs: vec![],
            protocols: vec![],
            agent_version: "test".to_string(),
            direction: "inbound".to_string(),
            rtt_ms: Some(42.5),
            last_seen_ms: Some(1712678400000),
            remote_nat_status: Some("public".to_string()),
            bandwidth_in: Some(1024),
            bandwidth_out: Some(2048),
        }],
        total: 1,
    };

    let json = serde_json::to_value(&list).unwrap();
    validate_against_schema("views/peer-list-view.schema.json", &json);
}
```

---

## Phase 7: Pre-push Hook Updates

### Task 7.1: Add views/ to schema-codegen trigger pattern

**File:** `.husky/pre-push`

The existing `schema-codegen` trigger matches `^elohim/sdk/schemas/` (line 143). This already covers `views/` schemas. No change needed — verify only.

### Task 7.2: Add schema contract test to elohim-storage gate

The elohim-storage gate (line 257–261 in pre-push fallback) already runs `cargo test`. The schema contract test is an integration test and runs with `cargo test --test schema_contract`. If the gate uses `cargo test` without filters, it includes integration tests automatically. Verify this is the case.

If the gate uses `--lib --bins` (which excludes integration tests), add `--test schema_contract` to the gate:

```bash
# In the elohim-storage fallback case:
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins --test schema_contract 2>&1
```

---

## Phase 8: Documentation

### Task 8.1: Update CLAUDE.md with schema contract pattern

**File:** `/projects/elohim/CLAUDE.md`

Add a new section after "Protocol Schema Validation" describing the view schema contract pattern:

```markdown
### View Schema Contract (HTTP wire shapes)
View schemas in `elohim/sdk/schemas/v1/views/` define the JSON wire format for HTTP API responses. They are the source of truth for the Rust→TypeScript boundary.

**Pattern:** Write the schema → Rust structs match it (hand-written with `#[serde(rename_all = "camelCase")]`) → validation harness (`tests/schema_contract.rs`) catches drift → TS codegen generates TypeScript interfaces.

**Conventions:** See `elohim/sdk/schemas/v1/views/CONVENTIONS.md` for the 10 rules.

**Adding a new view:**
1. Write `{name}.schema.json` in `elohim/sdk/schemas/v1/views/`
2. Write matching Rust struct in elohim-storage with `#[serde(rename_all = "camelCase")]`
3. Add schema contract test in `elohim/elohim-storage/tests/schema_contract.rs`
4. Add to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`
5. Run `pnpm run schema:codegen:ts` to generate and distribute TypeScript
6. Pre-push hook validates codegen freshness automatically
```

### Task 8.2: Update elohim-storage CLAUDE.md

**File:** `elohim/elohim-storage/CLAUDE.md`

Add a note about the schema contract in the "Adding New Entities" workflow:

```markdown
## Schema Contract (view validation)

View types in `views.rs` must match their JSON Schema in `../sdk/schemas/v1/views/`.
The `tests/schema_contract.rs` integration test validates this at `cargo test` time.

When modifying a View struct:
1. Update the schema first (`elohim/sdk/schemas/v1/views/{name}.schema.json`)
2. Update the Rust struct to match
3. Run `cargo test --test schema_contract` to verify
4. Run `pnpm run schema:codegen:ts` to regenerate TypeScript
```

---

## Phase 9: Sprint Commit

### Task 9.1: Run all quality gates

```bash
# Schema validation
pnpm run schema:codegen:ts -- --verify
pnpm run schema:codegen:rs -- --verify
pnpm run schema:validate

# Rust (elohim-storage)
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo fmt --check
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test

# Rust (doorway)
cd doorway/doorway-service
RUSTFLAGS="" cargo fmt --check
RUSTFLAGS="" cargo clippy -- -D warnings
RUSTFLAGS="" cargo test --lib --bins

# Angular
cd app/elohim-app
pnpm run lint
pnpm exec vitest run --config vite.config.ts
```

### Task 9.2: Create single sprint commit

Stage all changes and commit:

```
feat(p2p): establish view schema contract for P2P status and peer info

- Write JSON Schemas for P2PStatusView, PeerInfoView, PeerListView,
  DrainStatusView, ReplicationStatusView in elohim/sdk/schemas/v1/views/
- Add nat-status and relay-mode enum schemas
- Add validation harness (tests/schema_contract.rs) enforcing struct↔schema
  alignment at cargo test time
- Migrate P2PStatusInfo from snake_case to camelCase (schema-governed)
- Update all 6 consumers (doorway federation, connection indicator,
  simulate.sh, orchestrator Jenkinsfile, genesis Jenkinsfile, doorway health)
- Add GET /p2p/peers endpoint with Tier 1+2 PeerInfoView fields
- Extend codegen-ts.mjs to distribute new view types
- Write CONVENTIONS.md for views/ directory
- Fix economic-event-view.schema.json missing additionalProperties: false
```

---

## Deferred to Follow-up Sprint

The following Tier 3 plumbing requires libp2p API investigation and is scoped as a separate sprint:

1. **RTT ring buffer** — `ping::Event` handler → `HashMap<PeerId, VecDeque<Duration>>` → median RTT
2. **lastSeen tracking** — `ConnectionEvent` / `SwarmEvent` handler → `HashMap<PeerId, Instant>`
3. **remoteNatStatus** — autonat dial-back result storage
4. **Bandwidth sinks** — `BandwidthSinks` per-peer tracking (if libp2p 0.54 supports it)

All Tier 3 fields are already declared in the PeerInfoView schema as nullable. The follow-up sprint populates them.
