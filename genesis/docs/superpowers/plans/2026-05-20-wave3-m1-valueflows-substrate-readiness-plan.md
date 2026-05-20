# Wave 3 M1 — `bridges/valueflows` Substrate Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the substrate prerequisites for Wave 3: a new `bridges/` top-level directory containing the `valueflows-bridge` Rust crate (consumed by elohim-storage), the `/api/v1/vf-graphql` HTTP endpoint, the hREA DNA role added to the conductor's happ manifest, the `translation_observations` Diesel table for the learning ledger, and a tracer-bullet GET query for `EconomicEvent` returning fixture data through the full bridge → schema → response path.

**Architecture:** Three crates land in `bridges/valueflows/`: `valueflows-types` (stable type definitions like `TranslationPoint`), `valueflows-bridge` (library that mounts the route + holds the GraphQL schema), `valueflows-tests` (integration tests). Bridge consumed by `elohim-storage` via path dependency. hREA DNA is added as a new role in `elohim/holochain/dna/elohim/workdir/happ.yaml` with operator-fetched bundle (documented in README). The schema for M1 returns fixture data — real hREA reads + writes land in M2/M3.

**Tech Stack:** Rust 1.85+, async-graphql 7 (matches elohim-storage's existing version), hyper 1.6 (existing), Diesel 2 (existing), Holochain happ manifest v0 (existing).

**Reference spec:** `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`.

---

## File Structure

### Created in M1

**New top-level directory + workspace:**
- `bridges/CLAUDE.md` — high-level documentation of the bridges concept
- `bridges/valueflows/Cargo.toml` — Cargo workspace manifest (lists 3 members)
- `bridges/valueflows/README.md` — purpose, links to spec, dev workflow
- `bridges/valueflows/.gitignore` — `target/`, `Cargo.lock`

**`valueflows-types` crate (stable types):**
- `bridges/valueflows/valueflows-types/Cargo.toml`
- `bridges/valueflows/valueflows-types/src/lib.rs` — `TranslationPoint`, `TranslationKind`, `SemanticCost`, `OntologicalCommitment`, `Direction`, `ClientCapability` enums

**`valueflows-bridge` crate (library, mounts route + schema):**
- `bridges/valueflows/valueflows-bridge/Cargo.toml`
- `bridges/valueflows/valueflows-bridge/src/lib.rs` — `pub fn handle_request` entry point
- `bridges/valueflows/valueflows-bridge/src/schema/mod.rs` — `build_schema()` + `BridgeContext`
- `bridges/valueflows/valueflows-bridge/src/schema/economic_event.rs` — VF `EconomicEvent` GraphQL type + fixture resolver
- `bridges/valueflows/valueflows-bridge/src/ledger.rs` — write `TranslationPoint` to `translation_observations`

**`valueflows-tests` crate (integration tests):**
- `bridges/valueflows/valueflows-tests/Cargo.toml`
- `bridges/valueflows/valueflows-tests/src/lib.rs` — empty package marker
- `bridges/valueflows/valueflows-tests/tests/m1_tracer_bullet.rs` — end-to-end test

**hREA DNA scaffold:**
- `elohim/holochain/dna/hrea/workdir/README.md` — documents upstream fetch
- `elohim/holochain/dna/hrea/workdir/.gitignore` — ignores `*.dna` binaries

**`translation_observations` Diesel table:**
- `elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/up.sql`
- `elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/down.sql`
- `elohim/elohim-storage/src/db/translation_observations.rs` — Diesel model + `insert_observation`

### Modified in M1

- `elohim/elohim-storage/Cargo.toml` — add `valueflows-bridge = { path = "../../bridges/valueflows/valueflows-bridge" }`
- `elohim/elohim-storage/src/http.rs` — register `/api/v1/vf-graphql` route
- `elohim/elohim-storage/src/db/mod.rs` — `pub mod translation_observations;`
- `elohim/elohim-storage/src/db/schema.rs` — Diesel auto-regen will add `translation_observations` table
- `elohim/holochain/dna/elohim/workdir/happ.yaml` — add `hrea` role

---

## Task 1: Create `bridges/` top-level + `valueflows` workspace skeleton

**Files:**
- Create: `bridges/CLAUDE.md`
- Create: `bridges/valueflows/Cargo.toml`
- Create: `bridges/valueflows/README.md`
- Create: `bridges/valueflows/.gitignore`

- [ ] **Step 1: Create `bridges/CLAUDE.md`**

```markdown
# bridges/ — Pluggable Interop Layers

This directory holds bridge crates — Rust libraries that translate external
protocols (web2 federation, VF-GraphQL/hREA, future) to and from elohim's
canonical EPR-REA substrate.

## Pattern

Each bridge is a library crate. Runtimes (`doorway-service`, `elohim-storage`)
consume the bridges they need:

- `doorway-service` consumes bridges that absorb web2 traffic (`atproto`,
  `activitypub`, future)
- `elohim-storage` consumes bridges that speak protocol-shaped interop
  (`valueflows`)

Bridges are libraries, not services. The runtime hosting a bridge is decided
by the kind of traffic it absorbs (web2 = doorway; protocol = storage).

## Current bridges

- `valueflows/` — hREA / VF-GraphQL interop (Wave 3)

## Adding a new bridge

1. Create `bridges/<name>/` with its own Cargo workspace.
2. Expose a single library crate `<name>-bridge` with a `mount` or
   `handle_request` entry point.
3. Document which runtime consumes it.
4. Pull `qahal-authority` (from `elohim/qahal-authority`) if the bridge
   absorbs external writes.

## Reference spec

See `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
for the architectural pattern that produced this directory.
```

- [ ] **Step 2: Create `bridges/valueflows/Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "valueflows-types",
    "valueflows-bridge",
    "valueflows-tests",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
authors = ["Elohim Protocol"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1.43", features = ["full", "macros", "sync"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# GraphQL — match elohim-storage's version exactly
async-graphql = "7"

# HTTP types — match elohim-storage
hyper = { version = "1.6", features = ["http1", "http2", "server"] }
http-body-util = "0.1"
bytes = "1.10"

# Diesel — for ledger writes
diesel = { version = "2", features = ["sqlite", "r2d2", "chrono"] }

# Utilities
thiserror = "2.0"
anyhow = "1.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }

# Test utilities
tokio-test = "0.4"
```

- [ ] **Step 3: Create `bridges/valueflows/README.md`**

```markdown
# bridges/valueflows — hREA / VF-GraphQL Bridge

The bridge of VF/hREA → elohim EPR-REA. Sibling architectural pattern to
doorway's web2 → elohim P2P bridge.

## Consumed by

`elohim-storage` mounts this bridge at `/api/v1/vf-graphql`.

## Crates

- `valueflows-types` — stable type definitions (TranslationPoint, etc.)
- `valueflows-bridge` — library; GraphQL schema + handler
- `valueflows-tests` — integration tests against a mounted endpoint

## Build

```bash
cd bridges/valueflows
cargo check --all
cargo test --all
```

## Reference spec

`genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
```

- [ ] **Step 4: Create `bridges/valueflows/.gitignore`**

```
target/
Cargo.lock
```

- [ ] **Step 5: Verify workspace declared correctly**

Run: `cd /projects/elohim/bridges/valueflows && cargo check 2>&1 | tail -10`

Expected: `error: failed to load manifest for workspace member` (members don't exist yet — that's fine for this step; we verify the workspace file parses).

If the error is anything other than "member not found", investigate.

- [ ] **Step 6: Commit**

```bash
git add bridges/
git commit -m "scaffold(bridges): add bridges/ top-level + valueflows workspace skeleton

Per Wave 3 design spec — pluggable bridge libraries consumed by runtimes
(doorway for web2 bridges, elohim-storage for protocol bridges).

Workspace declared; member crates land in subsequent commits."
```

---

## Task 2: Create `valueflows-types` crate

**Files:**
- Create: `bridges/valueflows/valueflows-types/Cargo.toml`
- Create: `bridges/valueflows/valueflows-types/src/lib.rs`

- [ ] **Step 1: Write `bridges/valueflows/valueflows-types/Cargo.toml`**

```toml
[package]
name = "valueflows-types"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
description = "Stable type definitions for the valueflows bridge — TranslationPoint, enums."

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
```

- [ ] **Step 2: Write `bridges/valueflows/valueflows-types/src/lib.rs`**

```rust
//! Stable type definitions consumed by `valueflows-bridge` and any future
//! consumer of the bridge's learning-ledger schema. Kept in a separate crate
//! so the schema can be referenced by analysis tooling without pulling in
//! the full bridge (which depends on async-graphql + hyper).
//!
//! The ledger records each translation event — direction, VF type, semantic
//! cost — so we can produce an upstream-contribution inventory + R&O
//! compatibility report at M5.
//!
//! Reference spec:
//! `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
//! §4.2 (Learning Ledger Schema).

use serde::{Deserialize, Serialize};

/// One observation of the bridge translating between VF-GraphQL and elohim's
/// EPR-REA substrate. Written to the `translation_observations` Diesel table
/// in elohim-storage; aggregated at end-of-Wave-3 (M5) into the
/// upstream-contribution inventory + R&O compatibility report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationPoint {
    pub at_iso: String, // ISO-8601 UTC; chrono::Utc::now().to_rfc3339()
    pub direction: Direction,
    pub vf_type: String, // "EconomicEvent", "Proposal", ...
    pub elohim_source: String, // "hREA::EconomicEvent" | "elohim::EprAtom" | ...
    pub translation_kind: TranslationKind,
    pub semantic_cost: SemanticCost,
    pub ontological_commitment: Option<OntologicalCommitment>,
    pub client_capability: ClientCapability,
    pub code_location: String, // file:line, captured via macro
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TranslationKind {
    /// Identical shape — pure routing.
    IdentityShape,
    /// Shape identical, names differ.
    FieldRename,
    /// Genuine domain difference (Reach, ElohimAgent, ...).
    SemanticBridge,
    /// Same fact in two DHTs; merge for read.
    Reconciliation,
    /// Elohim-only data linked to canonical entry.
    Sidecar,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SemanticCost {
    /// Shape-equivalent translation; pure routing.
    Mechanical,
    /// Real semantic difference — keep distinct.
    JustifiedDistinct,
    /// Need more usage to judge.
    UnclearYet,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OntologicalCommitment {
    SovereigntyToStewardship,
    KeyAuthorityToSocialAuthority,
    FixedAudienceToReachClass,
    BilateralToRelational,
    IndividualWillToContribution,
    EntryToEprAtom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientCapability {
    /// Stock VF/hREA client; ignores elohim extension fields.
    StockVf,
    /// Client advertised support for `extensions.elohim.*` (via SDL
    /// `@elohim` directive or `X-Elohim-Extensions: 1` request header).
    ElohimAware,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_point_roundtrips_through_serde() {
        let p = TranslationPoint {
            at_iso: "2026-05-20T00:00:00Z".to_string(),
            direction: Direction::Read,
            vf_type: "EconomicEvent".to_string(),
            elohim_source: "fixture".to_string(),
            translation_kind: TranslationKind::IdentityShape,
            semantic_cost: SemanticCost::Mechanical,
            ontological_commitment: None,
            client_capability: ClientCapability::StockVf,
            code_location: "src/schema/economic_event.rs:42".to_string(),
            notes: None,
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let back: TranslationPoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn enums_serialize_as_strings() {
        let json = serde_json::to_value(Direction::Read).unwrap();
        assert_eq!(json, serde_json::json!("Read"));
        let json = serde_json::to_value(TranslationKind::SemanticBridge).unwrap();
        assert_eq!(json, serde_json::json!("SemanticBridge"));
    }
}
```

- [ ] **Step 3: Run unit tests**

Run: `cd /projects/elohim/bridges/valueflows && cargo test -p valueflows-types 2>&1 | tail -15`

Expected:
```
running 2 tests
test tests::enums_serialize_as_strings ... ok
test tests::translation_point_roundtrips_through_serde ... ok

test result: ok. 2 passed
```

- [ ] **Step 4: Commit**

```bash
git add bridges/valueflows/valueflows-types/
git commit -m "feat(valueflows-types): stable type defs for translation ledger

TranslationPoint + enums (TranslationKind, SemanticCost,
OntologicalCommitment, Direction, ClientCapability) per Wave 3
design §4.2. Standalone crate so analysis tooling can consume the
schema without pulling in async-graphql or hyper.

Serde-roundtrip + string-serialization tests pass."
```

---

## Task 3: Create `valueflows-bridge` crate skeleton with stub handler

**Files:**
- Create: `bridges/valueflows/valueflows-bridge/Cargo.toml`
- Create: `bridges/valueflows/valueflows-bridge/src/lib.rs`

- [ ] **Step 1: Write `bridges/valueflows/valueflows-bridge/Cargo.toml`**

```toml
[package]
name = "valueflows-bridge"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
description = "hREA / VF-GraphQL bridge — library mounted by elohim-storage at /api/v1/vf-graphql."

[dependencies]
valueflows-types = { path = "../valueflows-types" }

serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }

async-graphql = { workspace = true }
hyper = { workspace = true }
http-body-util = { workspace = true }
bytes = { workspace = true }

tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
```

- [ ] **Step 2: Write `bridges/valueflows/valueflows-bridge/src/lib.rs`**

```rust
//! valueflows-bridge — the hREA / VF-GraphQL bridge.
//!
//! Mounted by elohim-storage at `/api/v1/vf-graphql`. M1 ships a stub handler
//! that returns 503 unimplemented for any non-trivial query and fixture
//! `EconomicEvent` data for the M1 tracer-bullet query.
//!
//! See `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

pub mod schema;

/// Handle a single HTTP request arriving at `/api/v1/vf-graphql`.
///
/// Mirrors the existing `elohim-storage::graphql::server::handle_graphql` shape
/// so elohim-storage can route to either endpoint with the same pattern.
///
/// M1: returns 503 for any non-tracer-bullet query; fixture `EconomicEvent`
/// data for the M1 tracer-bullet query. M2+ will wire identity bridge,
/// authority gate, EPR atom emit, and real hREA projection.
pub async fn handle_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, BridgeError> {
    if req.method() != Method::POST {
        return Ok(error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "POST required for /api/v1/vf-graphql",
        ));
    }

    // Collect body bytes (same pattern as elohim-storage::graphql::server).
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| BridgeError::ReadBody(e.to_string()))?
        .to_bytes();

    let gql_request: async_graphql::Request = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_graphql_request",
                &format!("could not parse body as GraphQL request: {e}"),
            ));
        }
    };

    let schema = schema::build_schema();
    let gql_response = schema.execute(gql_request).await;

    let body = serde_json::to_vec(&gql_response)
        .map_err(|e| BridgeError::SerializeResponse(e.to_string()))?;

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| BridgeError::BuildResponse(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("could not read request body: {0}")]
    ReadBody(String),
    #[error("could not serialize response: {0}")]
    SerializeResponse(String),
    #[error("could not build response: {0}")]
    BuildResponse(String),
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "errors": [{
            "message": message,
            "extensions": { "code": code }
        }]
    });
    let body_bytes = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .expect("static response always builds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response_carries_expected_status_and_content_type() {
        let resp = error_response(
            StatusCode::BAD_REQUEST,
            "invalid_graphql_request",
            "boom",
        );
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers().get("content-type").unwrap().to_str().unwrap(),
            "application/json"
        );
    }

    // End-to-end request/response tests live in the valueflows-tests crate
    // (tests/m1_tracer_bullet.rs + tests/m1_http_smoke.rs) — hyper::body::Incoming
    // is not constructible directly, so a test helper (handle_request_for_test)
    // is added in Task 9 to exercise the parse → schema → response wire.
}
```

- [ ] **Step 3: Write `bridges/valueflows/valueflows-bridge/src/schema/mod.rs`**

```rust
//! GraphQL schema for the valueflows bridge.
//!
//! M1: minimal schema with `EconomicEvent` returning fixture data.
//! M2+ adds Agent + identity bridge.
//! M3+ adds Proposal/Intent + authority gate + real hREA projection.

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, ID};

pub mod economic_event;

pub use economic_event::EconomicEventGql;

/// M1 schema entry point. Empty mutation + subscription; queries return
/// fixture data only.
pub type BridgeSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema() -> BridgeSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish()
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Look up a VF EconomicEvent by id. M1 returns fixture data for any id.
    async fn economic_event(&self, id: ID) -> Option<EconomicEventGql> {
        Some(EconomicEventGql::fixture(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Request;

    #[tokio::test]
    async fn schema_serves_fixture_economic_event() {
        let schema = build_schema();
        let req = Request::new(
            r#"query { economicEvent(id: "test-id") { id action } }"#.to_string(),
        );
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "errors: {:?}", resp.errors);
        let data = resp.data.into_json().expect("data");
        assert_eq!(data["economicEvent"]["id"], "test-id");
        assert!(!data["economicEvent"]["action"].is_null());
    }
}
```

- [ ] **Step 4: Write `bridges/valueflows/valueflows-bridge/src/schema/economic_event.rs`**

```rust
//! VF `EconomicEvent` GraphQL object.
//!
//! M1: fixture resolver returns deterministic synthesized data. The fixture
//! shape matches VF's canonical `EconomicEvent` (see
//! `/projects/research/vf-graphql/lib/schemas/observation.gql`). M3+ replaces
//! the fixture with real reads from hREA DNA.

use async_graphql::{Object, ID};

/// VF EconomicEvent — minimal M1 surface. Canonical VF fields (per
/// `/projects/research/vf-graphql/lib/schemas/observation.gql`); elohim
/// extensions (Reach, sidecar links) land in M3+.
pub struct EconomicEventGql {
    pub id: String,
    pub action: String,        // VF action id (e.g., "transfer", "use")
    pub provider_id: String,   // VF Agent id (M1 fixture)
    pub receiver_id: String,   // VF Agent id (M1 fixture)
    pub note: Option<String>,
}

impl EconomicEventGql {
    /// Synthesize a fixture EconomicEvent for the M1 tracer bullet.
    ///
    /// The id passed in is echoed back so callers can verify the route
    /// is exercising the right resolver code path.
    pub fn fixture(id: String) -> Self {
        Self {
            id,
            action: "transfer".to_string(),
            provider_id: "agent-fixture-provider".to_string(),
            receiver_id: "agent-fixture-receiver".to_string(),
            note: Some("M1 tracer-bullet fixture; M3 will return real hREA data".to_string()),
        }
    }
}

#[Object]
impl EconomicEventGql {
    /// VF EconomicEvent identifier.
    async fn id(&self) -> ID {
        ID::from(self.id.clone())
    }

    /// VF action id describing the kind of event.
    async fn action(&self) -> &str {
        &self.action
    }

    /// VF Agent id of the provider.
    async fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// VF Agent id of the receiver.
    async fn receiver_id(&self) -> &str {
        &self.receiver_id
    }

    /// Optional free-form note.
    async fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}
```

- [ ] **Step 5: Run unit tests**

Run: `cd /projects/elohim/bridges/valueflows && cargo test -p valueflows-bridge 2>&1 | tail -20`

Expected:
```
running 1 test
test schema::tests::schema_serves_fixture_economic_event ... ok

running 1 test
test tests::error_response_carries_extensions_code ... ok

test result: ok
```

- [ ] **Step 6: Commit**

```bash
git add bridges/valueflows/valueflows-bridge/
git commit -m "feat(valueflows-bridge): library skeleton with fixture EconomicEvent

handle_request mirrors elohim-storage::graphql::server::handle_graphql
shape so the storage runtime can route to either endpoint identically.

M1 schema serves a fixture EconomicEvent for any id; M3 replaces with
real hREA reads. Unit tests cover schema execution and error response
shape."
```

---

## Task 4: Wire `valueflows-bridge` into `elohim-storage` as a dependency

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml` — add path dependency
- Modify: `elohim/elohim-storage/src/http.rs` — register route

- [ ] **Step 1: Add path dependency to `elohim/elohim-storage/Cargo.toml`**

Find the `[dependencies]` section (around line 11). After the existing `doorway-client = { path = "../../crates/doorway-client" }` line, add:

```toml
# Wave 3 — valueflows bridge (hREA / VF-GraphQL interop).
# See genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md
valueflows-bridge = { path = "../../bridges/valueflows/valueflows-bridge" }
```

- [ ] **Step 2: Find where elohim-storage routes are registered**

Run: `grep -nE 'Route::(get|post)\("/api/v1/' /projects/elohim/elohim/elohim-storage/src/http.rs | head -5`

Expected output gives you the line numbers where existing routes are declared. Pick a location alongside the other `/api/v1/*` routes — most cleanly near the existing graphql route registration.

- [ ] **Step 3: Find existing graphql route registration**

Run: `grep -nB1 -A4 'handle_graphql\|graphql/server\|"graphql"' /projects/elohim/elohim/elohim-storage/src/http.rs | head -30`

This shows you how `/api/v1/graphql` is currently wired. Mirror this pattern for `/api/v1/vf-graphql`. The exact insertion point depends on the existing structure — look for either:
- A `.route()` chain in a router-builder pattern, OR
- A `match (method, path)` dispatcher

The pattern in `http.rs` is the doorway-client `Route::get(...)/Route::post(...)` declarative pattern. Add a `Route::post("/api/v1/vf-graphql").handler("vf_graphql_handler").build()` line alongside the existing graphql route.

Then locate the actual handler dispatch (search for `"get_reciprocity"` handler-name dispatch we saw in L6). Add a handler arm that calls `valueflows_bridge::handle_request(req).await`.

- [ ] **Step 4: Add the route declaration**

Below the line declaring the `"/api/v1/graphql"` route (find via grep above), insert:

```rust
// Wave 3 M1 — valueflows bridge endpoint. Stub-stage: serves fixture
// EconomicEvent via the bridge's GraphQL schema. M2+ adds identity
// bridge, M3+ adds real hREA projection.
.route(
    Route::post("/api/v1/vf-graphql")
        .handler("vf_graphql_handler")
        .rate_limit(60)
        .build(),
)
```

- [ ] **Step 5: Add the handler dispatch arm**

In the handler match block (find via `grep -nE '"get_reciprocity"|"graphql_handler"' /projects/elohim/elohim/elohim-storage/src/http.rs`), add a new arm:

```rust
"vf_graphql_handler" => {
    valueflows_bridge::handle_request(req)
        .await
        .map_err(|e| StorageError::Internal(format!("vf-graphql bridge: {e}")))
}
```

- [ ] **Step 6: Build elohim-storage to verify wiring**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo build --features graph-native --lib 2>&1 | tail -20
```

Expected: `Finished `dev` profile [unoptimized + debuginfo] target(s)` — clean build.

If unresolved name errors mention `valueflows_bridge` or `handle_request`, re-check Cargo.toml dep + import. If the doorway-client `Route::post` API doesn't have `.handler()` or `.rate_limit()` methods, look at the existing graphql route declaration immediately above your insertion and copy its exact method-chain (the doorway-client API surface evolves).

- [ ] **Step 7: Commit**

```bash
git -C /projects/elohim add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/http.rs
git -C /projects/elohim commit -m "feat(elohim-storage): mount valueflows bridge at /api/v1/vf-graphql

Wave 3 M1 wiring — elohim-storage consumes valueflows-bridge as a
path dependency and registers /api/v1/vf-graphql with the
vf_graphql_handler dispatch arm.

Stub-stage: bridge serves fixture EconomicEvent. M2+ identity bridge,
M3+ real hREA projection."
```

---

## Task 5: Create `translation_observations` Diesel table

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/down.sql`
- Create: `elohim/elohim-storage/src/db/translation_observations.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Write the up migration**

`elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/up.sql`:

```sql
-- Wave 3 M1 — learning ledger for the valueflows bridge.
-- See genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md §4.2
--
-- Each row is one TranslationPoint observation. End-of-Wave-3 (M5) aggregates
-- these to produce the upstream-contribution inventory + R&O compatibility report.
CREATE TABLE translation_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observed_at TEXT NOT NULL,                  -- ISO-8601 UTC
    direction TEXT NOT NULL,                    -- 'Read' | 'Write'
    vf_type TEXT NOT NULL,                      -- 'EconomicEvent', 'Proposal', ...
    elohim_source TEXT NOT NULL,                -- 'fixture' | 'hREA::EconomicEvent' | ...
    translation_kind TEXT NOT NULL,             -- 'IdentityShape' | 'FieldRename' | ...
    semantic_cost TEXT NOT NULL,                -- 'Mechanical' | 'JustifiedDistinct' | 'UnclearYet'
    ontological_commitment TEXT,                -- nullable; enum string when set
    client_capability TEXT NOT NULL,            -- 'StockVf' | 'ElohimAware'
    code_location TEXT NOT NULL,                -- file:line
    notes TEXT                                  -- free-form
);

CREATE INDEX idx_translation_observations_observed_at ON translation_observations(observed_at);
CREATE INDEX idx_translation_observations_vf_type ON translation_observations(vf_type);
CREATE INDEX idx_translation_observations_kind_cost ON translation_observations(translation_kind, semantic_cost);
```

- [ ] **Step 2: Write the down migration**

`elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/down.sql`:

```sql
DROP INDEX IF EXISTS idx_translation_observations_kind_cost;
DROP INDEX IF EXISTS idx_translation_observations_vf_type;
DROP INDEX IF EXISTS idx_translation_observations_observed_at;
DROP TABLE IF EXISTS translation_observations;
```

- [ ] **Step 3: Write the Diesel model**

`elohim/elohim-storage/src/db/translation_observations.rs`:

```rust
//! Diesel model + insert helper for the `translation_observations` table.
//!
//! Schema lives in `src/db/schema.rs` (auto-regenerated by diesel print-schema).
//! See migration `2026-05-20-000000_translation_observations`.

use chrono::Utc;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use valueflows_types::{
    ClientCapability, Direction, OntologicalCommitment, SemanticCost, TranslationKind,
    TranslationPoint,
};

use crate::db::schema::translation_observations;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = translation_observations)]
pub struct TranslationObservationRow {
    pub id: i32,
    pub observed_at: String,
    pub direction: String,
    pub vf_type: String,
    pub elohim_source: String,
    pub translation_kind: String,
    pub semantic_cost: String,
    pub ontological_commitment: Option<String>,
    pub client_capability: String,
    pub code_location: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = translation_observations)]
pub struct NewTranslationObservation {
    pub observed_at: String,
    pub direction: String,
    pub vf_type: String,
    pub elohim_source: String,
    pub translation_kind: String,
    pub semantic_cost: String,
    pub ontological_commitment: Option<String>,
    pub client_capability: String,
    pub code_location: String,
    pub notes: Option<String>,
}

impl From<TranslationPoint> for NewTranslationObservation {
    fn from(p: TranslationPoint) -> Self {
        Self {
            observed_at: p.at_iso,
            direction: direction_string(p.direction).to_string(),
            vf_type: p.vf_type,
            elohim_source: p.elohim_source,
            translation_kind: translation_kind_string(p.translation_kind).to_string(),
            semantic_cost: semantic_cost_string(p.semantic_cost).to_string(),
            ontological_commitment: p
                .ontological_commitment
                .map(|o| ontological_commitment_string(o).to_string()),
            client_capability: client_capability_string(p.client_capability).to_string(),
            code_location: p.code_location,
            notes: p.notes,
        }
    }
}

/// Insert a TranslationPoint into the ledger.
pub fn insert_observation(
    conn: &mut SqliteConnection,
    point: TranslationPoint,
) -> Result<(), DieselError> {
    let row: NewTranslationObservation = point.into();
    diesel::insert_into(translation_observations::table)
        .values(&row)
        .execute(conn)?;
    Ok(())
}

/// Convenience: build a now-stamped TranslationPoint with the given fields.
pub fn observe_now(
    direction: Direction,
    vf_type: &str,
    elohim_source: &str,
    translation_kind: TranslationKind,
    semantic_cost: SemanticCost,
    ontological_commitment: Option<OntologicalCommitment>,
    client_capability: ClientCapability,
    code_location: &str,
) -> TranslationPoint {
    TranslationPoint {
        at_iso: Utc::now().to_rfc3339(),
        direction,
        vf_type: vf_type.to_string(),
        elohim_source: elohim_source.to_string(),
        translation_kind,
        semantic_cost,
        ontological_commitment,
        client_capability,
        code_location: code_location.to_string(),
        notes: None,
    }
}

fn direction_string(d: Direction) -> &'static str {
    match d {
        Direction::Read => "Read",
        Direction::Write => "Write",
    }
}

fn translation_kind_string(k: TranslationKind) -> &'static str {
    match k {
        TranslationKind::IdentityShape => "IdentityShape",
        TranslationKind::FieldRename => "FieldRename",
        TranslationKind::SemanticBridge => "SemanticBridge",
        TranslationKind::Reconciliation => "Reconciliation",
        TranslationKind::Sidecar => "Sidecar",
    }
}

fn semantic_cost_string(c: SemanticCost) -> &'static str {
    match c {
        SemanticCost::Mechanical => "Mechanical",
        SemanticCost::JustifiedDistinct => "JustifiedDistinct",
        SemanticCost::UnclearYet => "UnclearYet",
    }
}

fn ontological_commitment_string(o: OntologicalCommitment) -> &'static str {
    match o {
        OntologicalCommitment::SovereigntyToStewardship => "SovereigntyToStewardship",
        OntologicalCommitment::KeyAuthorityToSocialAuthority => "KeyAuthorityToSocialAuthority",
        OntologicalCommitment::FixedAudienceToReachClass => "FixedAudienceToReachClass",
        OntologicalCommitment::BilateralToRelational => "BilateralToRelational",
        OntologicalCommitment::IndividualWillToContribution => "IndividualWillToContribution",
        OntologicalCommitment::EntryToEprAtom => "EntryToEprAtom",
    }
}

fn client_capability_string(c: ClientCapability) -> &'static str {
    match c {
        ClientCapability::StockVf => "StockVf",
        ClientCapability::ElohimAware => "ElohimAware",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_point_converts_to_insert_row() {
        let p = observe_now(
            Direction::Read,
            "EconomicEvent",
            "fixture",
            TranslationKind::IdentityShape,
            SemanticCost::Mechanical,
            None,
            ClientCapability::StockVf,
            "test:1",
        );
        let row: NewTranslationObservation = p.into();
        assert_eq!(row.direction, "Read");
        assert_eq!(row.vf_type, "EconomicEvent");
        assert_eq!(row.translation_kind, "IdentityShape");
        assert_eq!(row.semantic_cost, "Mechanical");
        assert!(row.ontological_commitment.is_none());
    }
}
```

- [ ] **Step 4: Add the module to `src/db/mod.rs`**

Find `elohim/elohim-storage/src/db/mod.rs`. Find a `pub mod` line for an existing model (e.g., `pub mod peer_identity_bindings;` or `pub mod rea_commitments;`). Add alongside it:

```rust
pub mod translation_observations;
```

- [ ] **Step 5: Add elohim-storage dep on valueflows-types**

Find `elohim/elohim-storage/Cargo.toml`. Below the line you added in Task 4 Step 1 (`valueflows-bridge = { path = "..." }`), add:

```toml
valueflows-types = { path = "../../bridges/valueflows/valueflows-types" }
```

The translation_observations.rs module needs this for the `From<TranslationPoint>` impl.

- [ ] **Step 6: Apply the migration locally and regenerate schema.rs**

If a Diesel migration runner is set up:

```bash
cd /projects/elohim/elohim/elohim-storage
# Apply migrations against a temporary sqlite
DATABASE_URL=/tmp/elohim-storage-m1-migration-check.sqlite \
    diesel migration run 2>&1 | tail -10
# Regenerate schema.rs
DATABASE_URL=/tmp/elohim-storage-m1-migration-check.sqlite \
    diesel print-schema > src/db/schema.rs.new
diff src/db/schema.rs src/db/schema.rs.new | head -30
# If diff is sensible (adds the translation_observations table), replace:
mv src/db/schema.rs.new src/db/schema.rs
```

If no diesel CLI is set up, the schema regen happens via the embedded migrations runner at first startup. In that case, manually edit `src/db/schema.rs` to add the table block (mirror an adjacent table for syntax):

```rust
diesel::table! {
    translation_observations (id) {
        id -> Integer,
        observed_at -> Text,
        direction -> Text,
        vf_type -> Text,
        elohim_source -> Text,
        translation_kind -> Text,
        semantic_cost -> Text,
        ontological_commitment -> Nullable<Text>,
        client_capability -> Text,
        code_location -> Text,
        notes -> Nullable<Text>,
    }
}
```

- [ ] **Step 7: Build + run the module unit test**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --features graph-native --lib db::translation_observations 2>&1 | tail -15
```

Expected: `1 passed; 0 failed`.

- [ ] **Step 8: Commit**

```bash
git -C /projects/elohim add \
    elohim/elohim-storage/migrations/2026-05-20-000000_translation_observations/ \
    elohim/elohim-storage/src/db/translation_observations.rs \
    elohim/elohim-storage/src/db/mod.rs \
    elohim/elohim-storage/src/db/schema.rs \
    elohim/elohim-storage/Cargo.toml
git -C /projects/elohim commit -m "feat(elohim-storage): translation_observations table for valueflows ledger

Per Wave 3 design §4.2 — each row is one TranslationPoint observation
recorded by the valueflows bridge. M5 aggregates these into the
upstream-contribution inventory + R&O compatibility report.

Includes Diesel model, From<TranslationPoint> conversion, insert helper,
and observe_now() convenience constructor."
```

---

## Task 6: Wire the ledger insert into the EconomicEvent resolver

**Files:**
- Modify: `bridges/valueflows/valueflows-bridge/Cargo.toml` — depend on diesel + r2d2 pool re-export from valueflows-types? No — we'll pass the pool via async-graphql Context.
- Modify: `bridges/valueflows/valueflows-bridge/src/schema/mod.rs` — accept and store DbPool in context
- Modify: `bridges/valueflows/valueflows-bridge/src/schema/economic_event.rs` — write observation on resolve
- Modify: `bridges/valueflows/valueflows-bridge/src/lib.rs` — accept DbPool, pass into schema context

- [ ] **Step 1: Add the `elohim-storage` types we need (DbPool) to bridge deps**

`bridges/valueflows/valueflows-bridge/Cargo.toml` — extend `[dependencies]`:

```toml
diesel = { workspace = true }
r2d2 = "0.8"
```

(elohim-storage's DbPool is `r2d2::Pool<ConnectionManager<SqliteConnection>>` — we re-create the same type alias rather than depending on elohim-storage itself, since elohim-storage depends on us, not vice versa.)

- [ ] **Step 2: Define the BridgeContext type carrying the pool**

`bridges/valueflows/valueflows-bridge/src/schema/mod.rs` — replace the file with:

```rust
//! GraphQL schema for the valueflows bridge.
//!
//! M1: minimal schema with `EconomicEvent` returning fixture data + writing
//! a TranslationPoint observation to the learning ledger.

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, ID};
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;
use r2d2::Pool;

pub mod economic_event;

pub use economic_event::EconomicEventGql;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Context injected into the schema; held in async-graphql's data so resolvers
/// can grab the pool to log TranslationPoints.
#[derive(Clone)]
pub struct BridgeContext {
    pub pool: DbPool,
}

pub type BridgeSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(ctx: BridgeContext) -> BridgeSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(ctx)
        .finish()
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Look up a VF EconomicEvent by id. M1 returns fixture data and logs a
    /// TranslationPoint observation for every call.
    async fn economic_event(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: ID,
    ) -> async_graphql::Result<Option<EconomicEventGql>> {
        let bridge_ctx = ctx.data::<BridgeContext>()?;
        economic_event::resolve(bridge_ctx, id.to_string()).await
    }
}
```

- [ ] **Step 3: Replace `economic_event.rs` with resolver that writes the ledger**

`bridges/valueflows/valueflows-bridge/src/schema/economic_event.rs`:

```rust
//! VF `EconomicEvent` GraphQL object + M1 fixture resolver.
//!
//! Every resolve call writes a TranslationPoint observation to the ledger
//! so the M5 upstream-contribution + R&O compatibility reports have
//! coverage data even for the M1 tracer-bullet path.

use async_graphql::{Object, ID};
use chrono::Utc;
use diesel::prelude::*;
use valueflows_types::{
    ClientCapability, Direction, SemanticCost, TranslationKind, TranslationPoint,
};

use super::BridgeContext;

pub struct EconomicEventGql {
    pub id: String,
    pub action: String,
    pub provider_id: String,
    pub receiver_id: String,
    pub note: Option<String>,
}

impl EconomicEventGql {
    pub fn fixture(id: String) -> Self {
        Self {
            id,
            action: "transfer".to_string(),
            provider_id: "agent-fixture-provider".to_string(),
            receiver_id: "agent-fixture-receiver".to_string(),
            note: Some("M1 tracer-bullet fixture; M3 will return real hREA data".to_string()),
        }
    }
}

#[Object]
impl EconomicEventGql {
    async fn id(&self) -> ID {
        ID::from(self.id.clone())
    }
    async fn action(&self) -> &str {
        &self.action
    }
    async fn provider_id(&self) -> &str {
        &self.provider_id
    }
    async fn receiver_id(&self) -> &str {
        &self.receiver_id
    }
    async fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// M1 resolve: synthesize fixture, log observation, return.
pub async fn resolve(
    ctx: &BridgeContext,
    id: String,
) -> async_graphql::Result<Option<EconomicEventGql>> {
    let point = TranslationPoint {
        at_iso: Utc::now().to_rfc3339(),
        direction: Direction::Read,
        vf_type: "EconomicEvent".to_string(),
        elohim_source: "fixture".to_string(),
        translation_kind: TranslationKind::IdentityShape,
        semantic_cost: SemanticCost::Mechanical,
        ontological_commitment: None,
        client_capability: ClientCapability::StockVf,
        code_location: concat!(file!(), ":", line!()).to_string(),
        notes: Some("M1 tracer bullet — fixture resolver".to_string()),
    };

    // Fire-and-forget the ledger write; resolver succeeds even if ledger
    // insert fails (the response is the canonical artifact).
    if let Err(e) = write_observation(ctx, point) {
        tracing::warn!("translation_observations insert failed: {e}");
    }

    Ok(Some(EconomicEventGql::fixture(id)))
}

/// Insert the observation via a raw Diesel call. We inline the SQL here in
/// M1 to avoid a circular dependency with elohim-storage (which consumes
/// this crate). In M2+ this is refactored to call back into elohim-storage's
/// `db::translation_observations::insert_observation`.
fn write_observation(
    ctx: &BridgeContext,
    p: TranslationPoint,
) -> Result<(), diesel::result::Error> {
    use diesel::sql_query;
    let mut conn = ctx
        .pool
        .get()
        .map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;

    let ontological = p
        .ontological_commitment
        .map(|o| format!("{:?}", o))
        .unwrap_or_default();

    sql_query(
        r#"INSERT INTO translation_observations
        (observed_at, direction, vf_type, elohim_source, translation_kind,
         semantic_cost, ontological_commitment, client_capability,
         code_location, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind::<diesel::sql_types::Text, _>(p.at_iso)
    .bind::<diesel::sql_types::Text, _>(format!("{:?}", p.direction))
    .bind::<diesel::sql_types::Text, _>(p.vf_type)
    .bind::<diesel::sql_types::Text, _>(p.elohim_source)
    .bind::<diesel::sql_types::Text, _>(format!("{:?}", p.translation_kind))
    .bind::<diesel::sql_types::Text, _>(format!("{:?}", p.semantic_cost))
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
        if ontological.is_empty() { None } else { Some(ontological) },
    )
    .bind::<diesel::sql_types::Text, _>(format!("{:?}", p.client_capability))
    .bind::<diesel::sql_types::Text, _>(p.code_location)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(p.notes)
    .execute(&mut conn)?;
    Ok(())
}
```

- [ ] **Step 4: Update `lib.rs` to thread BridgeContext through**

`bridges/valueflows/valueflows-bridge/src/lib.rs` — modify `handle_request`:

```rust
// Replace the existing:
//     let schema = schema::build_schema();
// with:
pub async fn handle_request(
    req: Request<Incoming>,
    bridge_ctx: schema::BridgeContext,
) -> Result<Response<Full<Bytes>>, BridgeError> {
    if req.method() != Method::POST {
        return Ok(error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "POST required for /api/v1/vf-graphql",
        ));
    }

    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| BridgeError::ReadBody(e.to_string()))?
        .to_bytes();

    let gql_request: async_graphql::Request = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_graphql_request",
                &format!("could not parse body as GraphQL request: {e}"),
            ));
        }
    };

    let schema = schema::build_schema(bridge_ctx);
    let gql_response = schema.execute(gql_request).await;

    let body = serde_json::to_vec(&gql_response)
        .map_err(|e| BridgeError::SerializeResponse(e.to_string()))?;

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| BridgeError::BuildResponse(e.to_string()))
}
```

Re-export the context type:

```rust
// Add to lib.rs near the top:
pub use schema::{BridgeContext, DbPool};
```

- [ ] **Step 5: Update the schema unit test to pass a context**

Find the test in `schema/mod.rs` that calls `build_schema()`. Replace with a pool-aware version. Since constructing a real pool is heavy, gate the existing test to require a feature flag, and add a simpler "schema compiles" test:

In `schema/mod.rs`, replace the `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builds_with_empty_pool() {
        // Build a pool against an in-memory sqlite for the schema test.
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("build pool");
        let ctx = BridgeContext { pool };
        let _ = build_schema(ctx);
    }
}
```

Note: the test no longer runs a query because doing so requires the `translation_observations` table to exist in the in-memory pool. End-to-end exercise of the resolver happens in `valueflows-tests` (Task 8) where we set up the real schema.

- [ ] **Step 6: Update elohim-storage's handler arm to pass the pool**

In `elohim/elohim-storage/src/http.rs`, find the handler arm you added in Task 4 Step 5. Update to pass the BridgeContext:

```rust
"vf_graphql_handler" => {
    let bridge_ctx = valueflows_bridge::BridgeContext {
        pool: db_pool.clone(),  // exact name depends on the surrounding handler-dispatch scope
    };
    valueflows_bridge::handle_request(req, bridge_ctx)
        .await
        .map_err(|e| StorageError::Internal(format!("vf-graphql bridge: {e}")))
}
```

The name `db_pool` is illustrative — look at how an existing handler arm (e.g., `"get_reciprocity"`) accesses the pool, and mirror that exactly.

- [ ] **Step 7: Build elohim-storage**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo build --features graph-native --lib 2>&1 | tail -15
```

Expected: clean build. If type-mismatch errors mention `DbPool` differing between the bridge crate and elohim-storage, harmonize on `diesel::r2d2::Pool<ConnectionManager<SqliteConnection>>` exactly.

- [ ] **Step 8: Commit**

```bash
git -C /projects/elohim add bridges/valueflows/ elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/http.rs
git -C /projects/elohim commit -m "feat(valueflows-bridge): thread DbPool through context; log observations

EconomicEvent resolver writes a TranslationPoint to
translation_observations on every call. M1 fixture path provides
coverage data for the M5 evidence aggregator.

Handler arm in elohim-storage::http now constructs BridgeContext with
the storage's existing DbPool and passes it to the bridge."
```

---

## Task 7: Create `valueflows-tests` integration test crate

**Files:**
- Create: `bridges/valueflows/valueflows-tests/Cargo.toml`
- Create: `bridges/valueflows/valueflows-tests/src/lib.rs`
- Create: `bridges/valueflows/valueflows-tests/tests/m1_tracer_bullet.rs`

- [ ] **Step 1: Write `bridges/valueflows/valueflows-tests/Cargo.toml`**

```toml
[package]
name = "valueflows-tests"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
description = "Integration tests for the valueflows bridge."
publish = false

[dependencies]
valueflows-bridge = { path = "../valueflows-bridge" }
valueflows-types = { path = "../valueflows-types" }

tokio = { workspace = true }
async-graphql = { workspace = true }
serde_json = { workspace = true }
diesel = { workspace = true }
diesel_migrations = "2"
r2d2 = "0.8"

[dev-dependencies]
tokio-test = { workspace = true }
```

- [ ] **Step 2: Write `bridges/valueflows/valueflows-tests/src/lib.rs`**

```rust
//! Integration tests for the valueflows bridge.
//!
//! Real tests live in `tests/`. This file is the package marker.
```

- [ ] **Step 3: Write `bridges/valueflows/valueflows-tests/tests/m1_tracer_bullet.rs`**

```rust
//! M1 tracer-bullet integration test.
//!
//! Builds the bridge schema with a real (in-memory) DbPool, exercises the
//! economicEvent query, and asserts both:
//!   (a) the response contains fixture data
//!   (b) a row landed in translation_observations
//!
//! This is the smallest end-to-end test that proves the M1 wire path.

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use r2d2::Pool;
use valueflows_bridge::{schema, BridgeContext, DbPool};

/// We re-embed the elohim-storage migrations directory at test build time so
/// the in-memory sqlite has the `translation_observations` table.
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../../elohim/elohim-storage/migrations");

fn build_test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder()
        .max_size(1) // single conn so migrations + queries share state
        .build(manager)
        .expect("build pool");
    let mut conn = pool.get().expect("get conn");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("run migrations");
    pool
}

#[tokio::test]
async fn economic_event_query_returns_fixture_and_logs_observation() {
    let pool = build_test_pool();
    let ctx = BridgeContext { pool: pool.clone() };
    let schema = schema::build_schema(ctx);

    let req = async_graphql::Request::new(
        r#"query { economicEvent(id: "tracer-bullet-id") {
              id action providerId receiverId note
          } }"#
            .to_string(),
    );
    let resp = schema.execute(req).await;
    assert!(resp.errors.is_empty(), "graphql errors: {:?}", resp.errors);

    let data = resp.data.into_json().expect("data is json");
    let ee = &data["economicEvent"];
    assert_eq!(ee["id"], "tracer-bullet-id", "fixture echoes id");
    assert_eq!(ee["action"], "transfer", "fixture action");
    assert_eq!(ee["providerId"], "agent-fixture-provider");
    assert_eq!(ee["receiverId"], "agent-fixture-receiver");
    assert!(ee["note"].is_string(), "note present");

    // Verify a translation observation was written.
    let mut conn = pool.get().expect("get conn");
    let count: i64 = diesel::sql_query("SELECT COUNT(*) AS c FROM translation_observations")
        .get_result::<CountRow>(&mut conn)
        .expect("count query")
        .c;
    assert_eq!(count, 1, "exactly one observation written");

    let kind: String = diesel::sql_query(
        "SELECT translation_kind AS c FROM translation_observations LIMIT 1",
    )
    .get_result::<StringRow>(&mut conn)
    .expect("kind query")
    .c;
    assert_eq!(kind, "IdentityShape", "M1 fixture is IdentityShape");
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    c: i64,
}

#[derive(QueryableByName)]
struct StringRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    c: String,
}
```

- [ ] **Step 4: Run the integration test**

```bash
cd /projects/elohim/bridges/valueflows
cargo test -p valueflows-tests 2>&1 | tail -20
```

Expected:
```
running 1 test
test economic_event_query_returns_fixture_and_logs_observation ... ok

test result: ok. 1 passed; 0 failed
```

If the test fails because `embed_migrations!` can't find the path, check the relative path in `MIGRATIONS = embed_migrations!(...)`. The path is relative to the package's `Cargo.toml`, not to the source file.

- [ ] **Step 5: Commit**

```bash
git -C /projects/elohim add bridges/valueflows/valueflows-tests/
git -C /projects/elohim commit -m "test(valueflows): M1 tracer-bullet integration test

End-to-end exercise: build schema with real DbPool, run economicEvent
query, assert fixture response shape AND a translation_observations row
landed.

Migrations are embedded from elohim/elohim-storage/migrations/ so the
test runs against the same schema as production. In-memory sqlite
pool keeps the test self-contained."
```

---

## Task 8: Add hREA DNA role scaffold to happ manifest

**Files:**
- Create: `elohim/holochain/dna/hrea/workdir/README.md`
- Create: `elohim/holochain/dna/hrea/workdir/.gitignore`
- Modify: `elohim/holochain/dna/elohim/workdir/happ.yaml`

- [ ] **Step 1: Create the hrea workdir directory**

```bash
mkdir -p /projects/elohim/elohim/holochain/dna/hrea/workdir
```

- [ ] **Step 2: Write `elohim/holochain/dna/hrea/workdir/README.md`**

```markdown
# hREA DNA — workdir

This directory holds the **external** hREA DNA bundle. hREA is published
by the Holochain hREA team (Lynn Foster, Bob Haugen, et al.); we consume
it as a versioned binary, not by building it ourselves.

## Wave 3 design context

Wave 3 of the cross-wave guidance adds hREA as a projection target for
VF-shaped writes. The valueflows bridge (at `bridges/valueflows/`)
translates VF-GraphQL queries and mutations and projects them into hREA
entries via per-Human cells provisioned during the VFBinding handshake.

See:
- `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
- `https://github.com/h-REA/hREA/releases`

## Fetching the bundle

The `hrea.dna` binary is **not in git** (it's a built artifact from
upstream releases). To populate this directory:

```bash
# From repo root. Replace VERSION with the version pinned in happ.yaml.
VERSION=$(grep -A 2 "name: hrea" elohim/holochain/dna/elohim/workdir/happ.yaml \
            | grep "version_pin" | awk '{print $2}' | tr -d '"')
mkdir -p elohim/holochain/dna/hrea/workdir
curl -L -o elohim/holochain/dna/hrea/workdir/hrea.dna \
    "https://github.com/h-REA/hREA/releases/download/${VERSION}/hrea.dna"
```

If the upstream URL changes, update both this README and the happ.yaml
`version_pin` comment.

## Currently pinned version

See `dna.path` in `elohim/holochain/dna/elohim/workdir/happ.yaml`.
```

- [ ] **Step 3: Write `elohim/holochain/dna/hrea/workdir/.gitignore`**

```
*.dna
*.happ
```

- [ ] **Step 4: Add the hREA role to `elohim/holochain/dna/elohim/workdir/happ.yaml`**

Find the end of the `roles:` list. After the last existing role (look for `- name: node-registry` or similar; verify the last role's name), add:

```yaml
  # hREA DNA — Wave 3 substrate readiness.
  # Lynn Foster's canonical hREA (h-REA/hREA). Pinned to a specific upstream
  # release; bundle fetched out-of-band into ../hrea/workdir/hrea.dna.
  # See genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md
  # version_pin: 0.0.0  <-- update when first real version is pinned
  - name: hrea
    provisioning:
      strategy: create
      deferred: true   # cells provisioned lazily in M2 (VFBinding handshake)
    dna:
      path: "../hrea/workdir/hrea.dna"
      modifiers:
        network_seed: ~
        properties: ~
        origin_time: ~
        quantum_time: ~
      installed_hash: ~
      clone_limit: 0
```

The `deferred: true` is critical: hREA cells are provisioned lazily per Human during the VFBinding handshake (M2), not at conductor startup. Without `deferred`, conductor startup fails when the binary isn't present.

- [ ] **Step 5: Validate the manifest**

If you have `hc` (Holochain CLI) available in your shell:

```bash
cd /projects/elohim/elohim/holochain/dna/elohim/workdir
hc app validate happ.yaml 2>&1 | head -20
```

Expected: success message OR an error pointing at hREA bundle missing. If the latter, the manifest is valid — we just don't have the binary yet (acceptable for M1 since `deferred: true`).

If `hc` is not available in your shell, skip this step — Jenkins CI runs the validation.

- [ ] **Step 6: Commit**

```bash
git -C /projects/elohim add elohim/holochain/dna/hrea/workdir/ \
                              elohim/holochain/dna/elohim/workdir/happ.yaml
git -C /projects/elohim commit -m "feat(happs): add hrea DNA role to elohim happ (deferred provisioning)

Wave 3 substrate readiness. hREA bundle fetched out-of-band into
hrea/workdir/ (gitignored). Role uses deferred: true so cells provision
lazily per Human during the VFBinding handshake (M2), not at conductor
startup.

See genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md"
```

---

## Task 9: End-to-end smoke test against a running elohim-storage

**Files:**
- Create: `bridges/valueflows/valueflows-tests/tests/m1_http_smoke.rs`

- [ ] **Step 1: Write the HTTP-level smoke test**

This test starts elohim-storage in-process (or hits a running instance) and POSTs a real HTTP request. It complements the schema-level test from Task 7.

`bridges/valueflows/valueflows-tests/tests/m1_http_smoke.rs`:

```rust
//! M1 HTTP-level smoke test.
//!
//! Goes one layer deeper than the schema test (Task 7): exercises the
//! hyper-level handle_request() entry point with a real Request/Body,
//! proving the route → handler → schema wire is intact.

use bytes::Bytes;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use http_body_util::Full;
use hyper::{Method, Request};
use r2d2::Pool;
use valueflows_bridge::{handle_request, BridgeContext, DbPool};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../../elohim/elohim-storage/migrations");

fn build_test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder().max_size(1).build(manager).expect("build pool");
    let mut conn = pool.get().expect("get conn");
    conn.run_pending_migrations(MIGRATIONS).expect("migrations");
    pool
}

#[tokio::test]
async fn post_vf_graphql_returns_fixture_economic_event() {
    let pool = build_test_pool();
    let ctx = BridgeContext { pool };

    let body = serde_json::json!({
        "query": "query { economicEvent(id: \"smoke-test-id\") { id action } }",
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    // hyper::body::Incoming is not constructible from raw bytes outside
    // hyper's internals. Bridge handle_request takes Request<Incoming>; the
    // test uses Request<Full<Bytes>> via the test-friendly conversion:
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/vf-graphql")
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .expect("build request");

    // Note: handle_request signature uses Request<Incoming>. We need a
    // thin shim or the test crate must use a different entry point.
    // For M1, we test the schema directly (Task 7 covers that path); HTTP
    // wire test is gated behind a feature flag or skipped pending
    // dispatcher integration.
    //
    // Confirm the request builds:
    assert_eq!(req.method(), Method::POST);
    assert_eq!(req.uri().path(), "/api/v1/vf-graphql");
}
```

- [ ] **Step 2: Acknowledge the hyper-Incoming limitation**

The above test stops short of actually calling `handle_request` because `hyper::body::Incoming` can't be constructed directly in tests. Two options to address in M1:

a. Add a test-helper entry point `handle_request_from_bytes(bytes, ctx)` to `valueflows-bridge::lib.rs` that bypasses Incoming and runs the same logic.
b. Defer this test to when elohim-storage runs end-to-end (Jenkins integration).

For M1, do (a): add a test helper.

In `bridges/valueflows/valueflows-bridge/src/lib.rs`, add:

```rust
/// Test helper: same as `handle_request` but accepts raw bytes instead of
/// `Request<Incoming>`. Used by valueflows-tests; not exposed via the
/// production handler.
#[doc(hidden)]
pub async fn handle_request_for_test(
    body_bytes: bytes::Bytes,
    bridge_ctx: schema::BridgeContext,
) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, BridgeError> {
    let gql_request: async_graphql::Request = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return Ok(error_response(
                hyper::StatusCode::BAD_REQUEST,
                "invalid_graphql_request",
                &format!("could not parse body as GraphQL request: {e}"),
            ));
        }
    };
    let schema = schema::build_schema(bridge_ctx);
    let gql_response = schema.execute(gql_request).await;
    let body = serde_json::to_vec(&gql_response)
        .map_err(|e| BridgeError::SerializeResponse(e.to_string()))?;
    hyper::Response::builder()
        .status(hyper::StatusCode::OK)
        .header("content-type", "application/json")
        .body(http_body_util::Full::new(bytes::Bytes::from(body)))
        .map_err(|e| BridgeError::BuildResponse(e.to_string()))
}
```

- [ ] **Step 3: Update the smoke test to use the helper**

Replace the body of `m1_http_smoke.rs` with:

```rust
use bytes::Bytes;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use http_body_util::BodyExt;
use r2d2::Pool;
use valueflows_bridge::{handle_request_for_test, BridgeContext, DbPool};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../../elohim/elohim-storage/migrations");

fn build_test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder().max_size(1).build(manager).expect("build pool");
    let mut conn = pool.get().expect("get conn");
    conn.run_pending_migrations(MIGRATIONS).expect("migrations");
    pool
}

#[tokio::test]
async fn vf_graphql_returns_fixture_economic_event_via_handler_for_test() {
    let pool = build_test_pool();
    let ctx = BridgeContext { pool };

    let body = serde_json::json!({
        "query": "query { economicEvent(id: \"smoke\") { id action providerId } }",
    });
    let body_bytes = Bytes::from(serde_json::to_vec(&body).unwrap());

    let resp = handle_request_for_test(body_bytes, ctx)
        .await
        .expect("handler returns OK response");

    assert_eq!(resp.status(), 200);

    let resp_body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp_body).expect("parse response json");

    assert!(
        resp_json["errors"].is_null() || resp_json["errors"].as_array().unwrap().is_empty(),
        "no graphql errors: {:?}",
        resp_json["errors"]
    );
    let ee = &resp_json["data"]["economicEvent"];
    assert_eq!(ee["id"], "smoke");
    assert_eq!(ee["action"], "transfer");
    assert_eq!(ee["providerId"], "agent-fixture-provider");
}
```

- [ ] **Step 4: Run both tests**

```bash
cd /projects/elohim/bridges/valueflows
cargo test -p valueflows-tests 2>&1 | tail -20
```

Expected:
```
running 1 test (m1_tracer_bullet)
test economic_event_query_returns_fixture_and_logs_observation ... ok

running 1 test (m1_http_smoke)
test vf_graphql_returns_fixture_economic_event_via_handler_for_test ... ok

test result: ok. 2 passed
```

- [ ] **Step 5: Commit**

```bash
git -C /projects/elohim add bridges/valueflows/
git -C /projects/elohim commit -m "test(valueflows-tests): HTTP-level smoke test via handle_request_for_test

Adds handle_request_for_test helper that bypasses the hyper::Incoming
construction limitation (which can't be built from raw bytes in tests).
Same logic as handle_request; production path unchanged.

Smoke test exercises the body-bytes → GraphQL parse → schema execute
→ response serialize path with a real DbPool. Complements the schema-
level test from m1_tracer_bullet.rs."
```

---

## Task 10: Document and verify M1 complete

**Files:**
- Create: `bridges/valueflows/CLAUDE.md`

- [ ] **Step 1: Write `bridges/valueflows/CLAUDE.md`**

```markdown
# bridges/valueflows — Local Guidance

The hREA / VF-GraphQL bridge for the Elohim Protocol. Consumed by
`elohim-storage`; mounted at `/api/v1/vf-graphql`.

## Workspace structure

- `valueflows-types/` — stable type definitions (TranslationPoint, enums).
  Standalone crate so analysis tooling can depend on the ledger schema
  without pulling async-graphql + hyper.
- `valueflows-bridge/` — library; GraphQL schema + handler entry point.
- `valueflows-tests/` — integration tests (schema-level + HTTP-level).

## Current state (M1)

- `/api/v1/vf-graphql` mounted on elohim-storage.
- `EconomicEvent` query returns deterministic fixture data.
- Every resolve writes a `TranslationPoint` to the
  `translation_observations` table.
- hREA DNA role added to happ manifest with `deferred: true` (cells
  provision lazily in M2).
- No mutations yet.

## Reference docs

- Spec: `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
- M1 plan: `genesis/docs/superpowers/plans/2026-05-20-wave3-m1-valueflows-substrate-readiness-plan.md`

## Build / test

```bash
cd bridges/valueflows
cargo check --all
cargo test --all
```

For elohim-storage integration, use the storage workspace's build:

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo build --features graph-native --lib
```

## Sequencing

- M1 (this) — substrate, fixture EconomicEvent.
- M2 — identity bridge: VfBinding entry, handshake, per-Human hREA cells,
  `elohim/qahal-authority` crate.
- M3 — authority gate + write path for Proposal+Intent.
- M4 — remaining VF types.
- M5 — learning ledger reports.
- M6 (optional) — Apollo Federation.
```

- [ ] **Step 2: Run the full bridge test suite**

```bash
cd /projects/elohim/bridges/valueflows
cargo test --all 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 3: Build elohim-storage with the new dep**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo build --features graph-native --lib 2>&1 | tail -10
```

Expected: clean build.

- [ ] **Step 4: Run the existing elohim-storage test suite to verify no regressions**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --features graph-native --lib 2>&1 | tail -15
```

Expected: all existing tests still pass; one new test (`db::translation_observations::tests::translation_point_converts_to_insert_row`).

- [ ] **Step 5: Commit**

```bash
git -C /projects/elohim add bridges/valueflows/CLAUDE.md
git -C /projects/elohim commit -m "docs(valueflows): M1 complete — substrate readiness landed

bridges/valueflows workspace created, valueflows-bridge mounted at
/api/v1/vf-graphql in elohim-storage, EconomicEvent fixture path
working end-to-end with translation_observations ledger writes,
hREA DNA role added to happ manifest with deferred provisioning.

CLAUDE.md documents the workspace + current M1 state + sequencing
for M2-M6."
```

---

## Out of scope (call-outs)

- **No real hREA reads or writes.** M1 returns fixture EconomicEvent data. M2 builds the identity bridge that lets real per-Human hREA cells exist; M3 wires actual hREA writes through the authority gate.
- **No mutations.** Only `economicEvent(id)` query in M1.
- **No `qahal-authority` crate yet.** Lives in `elohim/qahal-authority/` and lands in M2.
- **No identity bridge.** No `VfBinding`, no handshake, no per-Human cells. M2.
- **No extensions.elohim.* response fields.** Vanilla VF response shapes only. M3.
- **No reconciliation worker.** No projections exist to reconcile. M3.
- **No upstream hREA binary in git.** Operators fetch out-of-band per `elohim/holochain/dna/hrea/workdir/README.md`. Production CI workflows need to fetch the bundle before building the happ.
- **No R&O integration test.** R&O compat smoke lands once mutations work (M3+).

## Self-review

**Spec coverage:** every M1 requirement in the spec's §7 milestone has a task:
- "Add hREA DNA to conductor (version-pinned)" → Task 8
- "Create `bridges/valueflows` workspace skeleton" → Tasks 1, 2, 3, 7
- "Mount empty `/api/v1/vf-graphql` route on elohim-storage" → Task 4
- "Ship VF read endpoint for one type (EconomicEvent) end-to-end as a tracer bullet" → Tasks 6, 9
- "Learning ledger schema lands" → Task 5
- "Translation observation written for every read" → Tasks 6, 7

**Placeholder scan:** code blocks contain complete code; no "TODO" / "TBD" / "implement later" markers in steps. Where a step has uncertainty (e.g., exact line in http.rs for route registration), it directs the engineer to a grep command to locate the right insertion point — concrete instruction, not a placeholder.

**Type consistency:** `TranslationPoint`, `BridgeContext`, `DbPool`, `EconomicEventGql`, `NewTranslationObservation` are used consistently across tasks. `DbPool` is `r2d2::Pool<ConnectionManager<SqliteConnection>>` in both `valueflows-bridge::schema` and `elohim-storage`. `BridgeContext` carries the pool field uniformly.

## Execution handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-05-20-wave3-m1-valueflows-substrate-readiness-plan.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration with two-stage review per task.

2. **Inline Execution** — run all tasks in this session using `superpowers:executing-plans`, batched with checkpoints for operator review.

Which approach?
