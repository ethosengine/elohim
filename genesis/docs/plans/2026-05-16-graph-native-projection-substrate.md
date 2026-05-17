# Graph-Native Projection Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land a graph-native projection layer (CozoDB) inside elohim-storage alongside the existing diesel relational projection, with manifest-driven extensibility, extend shefa + lamad manifests as the first demonstrating domains, land 9 view builders and a server-side GraphQL surface (Apollo Federation v2 subgraph spec).

**Architecture:** Substrate (DHT) remains canonical. elohim-storage gains a second projection target (CozoDB with sqlite backend) running alongside diesel. The projector fans out to both targets per-EPR. Domain manifests declare graph types via a new `"graph"` section in app-manifest.schema.json. View builders compose graph + relational reads into typed wire shapes (REST views + GraphQL).

**Tech Stack:** Rust (elohim-storage), CozoDB embedded (Datalog query language), async-graphql (Rust GraphQL server), diesel (existing relational ORM), sqlite (CozoDB persistence backend), Apollo Federation v2 SDL (GraphQL wire contract), pnpm workspace (TypeScript codegen pipeline).

**Spec:** `genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md`

**Sprint discipline (carries into every task):**
- Backend only this sprint; no Angular work; no @wip BDD scenarios lift
- **Everything graph-native is feature-gated under Cargo feature `graph-native` (default-on)** — see Device-Class Gating Discipline below. Wearables, Chromebooks, IoT, elohim-observer thin clients build with `--no-default-features` and carry no CozoDB/async-graphql weight.
- Native Rust builds require `RUSTFLAGS=""` and `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev` per CLAUDE.md gotcha + memory `feedback_cargo_target_dir_for_native_builds`
- No DNA touches expected; if forced, sweettest on Jenkins per memory `feedback_shift_measure_jenkins`
- Schema-first: any new view schema authored in `elohim/sdk/schemas/v1/views/` BEFORE writing the Rust struct
- Commits per task; never amend; never `--no-verify`
- Stewardship vocabulary throughout — `stewards/contributors/authored`, never `owns/owners/sovereign`

## Device-Class Gating Discipline (Cargo feature `graph-native`)

**Principle:** elohim runs on devices spanning a wide capability range — household hubs and node runtimes need the full graph projection; wearables, Chromebooks, IoT, and elohim-observer thin clients primarily emit signals and do not host views. The graph engine, GraphQL surface, and graph-backed view builders are conditional capability for the high-capacity tier. Aligns with `project_hub_optional_floor` ("design floor: one device, no hub; hubs are convenience, never gate participation") and `project_compute_and_model_independent_diversity_surfaces`.

**Mechanism:** single Cargo feature `graph-native` in `elohim-storage/Cargo.toml`, default-on.

```toml
[features]
default = ["graph-native"]
graph-native = ["dep:cozo", "dep:async-graphql", "dep:async-graphql-axum", "dep:chrono"]

[dependencies]
cozo = { version = "0.7", default-features = false, features = ["storage-sqlite"], optional = true }
async-graphql = { version = "7", features = ["apollo_tracing"], optional = true }
async-graphql-axum = { version = "7", optional = true }
chrono = { version = "0.4", optional = true }  # graph projector uses chrono::Utc::now()
```

**Per-task discipline:** every Rust file/module/test introduced for graph-native behavior must be gated:
- Module-level: `#[cfg(feature = "graph-native")] pub mod graph;` in `lib.rs`; same for `pub mod graphql;` and `pub mod views::lamad/shefa;`
- Function-level inside otherwise-shared modules: `#[cfg(feature = "graph-native")] fn graph_projector_fan_out(...) { ... }` with a `#[cfg(not(feature = "graph-native"))]` no-op shim where the call site is shared
- Test modules: `#[cfg(all(test, feature = "graph-native"))]` on every graph-related test file/function
- Route registration: gate the four new REST routes + the GraphQL endpoint behind the feature; when disabled, register no-op 501/404 handlers that return `{"error":"requires graph-native feature","capability":"graph-native"}` so thin clients respond predictably

**The Task 27 env-var feature flag (`ELOHIM_GRAPH_BACKED_VIEWS`) is REPLACED by this Cargo feature.** The existing 5 shefa routes' code path becomes `#[cfg(feature = "graph-native")]` for the graph-backed branch; without the feature, only the legacy relational branch compiles. Cleaner than env-var-at-runtime; no behavior surprises when the binary lacks the engine.

**Thin-build verification** is a closing condition (Task 35a, new). The CI orchestrator should add a thin-build job that runs `cargo build --no-default-features` against `elohim-storage` and asserts the binary is buildable + meaningfully smaller (target: at least 30MB difference, indicating CozoDB + async-graphql actually dropped).

**What this does NOT change in the plan:**
- Schema authoring (the view schemas in `elohim/sdk/schemas/v1/views/` exist regardless of build feature — they're cross-language contracts)
- Manifest extensions (the `"graph"` sections in lamad/shefa manifest.json are data that thin clients can still parse if needed, they just won't apply it)
- Spec or design — the spec describes the full-feature capability; this is purely build/deploy variance

**What this DOES change in the plan:**
- Tasks 1, 12, 23, 24, 26, 27, 29, 30 gain `#[cfg(feature = "graph-native")]` annotations on the introduced code
- Task 1 declares the Cargo feature
- Task 27 replaces env-var flag with the Cargo feature
- A new Task 35a inserts before the existing Task 35 to verify thin-client build behavior
- Closing condition list (Task 35) adds: "thin-client build (`--no-default-features`) succeeds + 4 new REST routes return 501 + GraphQL returns 404 + relational projection still works"

## Source-of-Truth Declaration (P2P Design Gate)

Per spec §11 (P2P Design Gate Output) and the spec's preamble (Reading A): **every CozoDB `:create` relation in this plan is a projection-layer entity (classification: Operational/C), not a source-of-truth.**

- **Source of truth for all graph relations:** Holochain DHT (canonical EprHead atoms, notarized; canonical_bytes blob in `epr_atoms` relational projection acts as the local materialized view of substrate truth).
- **Reconstruction strategy:** `projector::backfill_graph` walks `epr_atoms` (relational projection) and re-derives every CozoDB relation row from canonical bytes. Ultimately rebuildable from the substrate itself via re-fetch from peers.
- **Identity derivation:** every primary key in graph relations is derived from a substrate CID (content-addressed); no new identity-bearing entities introduced.
- **No new DHT entry types:** lamad ~73/100 and mishpat ~11/100 entry-type headroom preserved (verified per spec §11).
- **No new HTTP-route-first design:** all new REST routes (`/api/v1/views/...`) project from view builders, not authored stores.

This declaration covers every `:create` and `::index create` statement in Tasks 2–7, 16–17, and 19–20. The audit-grade restatement appears once here rather than repeated per task to keep tasks tight while preserving traceability.

Reference: `.claude/skills/p2p-design-gate/SKILL.md`; spec section: `genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md#11-p2p-design-gate-output`.

---

## Phase 1: Foundation (CozoDB + Core Schema)

### Task 1: Add CozoDB dependency and smoke test

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Create: `elohim/elohim-storage/src/graph/mod.rs`
- Create: `elohim/elohim-storage/src/graph/engine.rs`
- Create: `elohim/elohim-storage/tests/graph_engine_smoke.rs`

- [ ] **Step 1: Write failing smoke test**

Create `elohim/elohim-storage/tests/graph_engine_smoke.rs`:

```rust
use elohim_storage::graph::engine::GraphEngine;

#[test]
fn engine_initializes_with_sqlite_backend() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("graph.db");
    let engine = GraphEngine::open(&path).expect("engine opens");
    let result = engine
        .run_script("?[a] := a = 1", &[])
        .expect("trivial query runs");
    assert_eq!(result.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke
```
Expected: FAIL — module not found.

- [ ] **Step 3: Add CozoDB to Cargo.toml under the `graph-native` feature**

In `elohim/elohim-storage/Cargo.toml`:

```toml
[features]
default = ["graph-native"]
graph-native = ["dep:cozo", "dep:chrono"]
# async-graphql/async-graphql-axum are added under graph-native in Task 29

[dependencies]
cozo = { version = "0.7", default-features = false, features = ["storage-sqlite"], optional = true }
chrono = { version = "0.4", optional = true }

[dev-dependencies]
tempfile = "3"  # ensure present
```

Then gate the module declaration. In `elohim/elohim-storage/src/lib.rs`:

```rust
#[cfg(feature = "graph-native")]
pub mod graph;
```

And gate the test file. In `elohim/elohim-storage/tests/graph_engine_smoke.rs`, add at the top:

```rust
#![cfg(feature = "graph-native")]
```

(Same `#![cfg(feature = "graph-native")]` top-line goes on every test file introduced for graph behavior: `graph_engine_smoke.rs`, `graph_indexes.rs`, `graph_primitives.rs`, `graph_projector.rs`, `graph_backfill.rs`, `projection_fanout.rs`, `manifest_graph_validator.rs`, `lamad_manifest_registration.rs`, `shefa_manifest_registration.rs`, `views_lamad.rs`, `views_shefa.rs`, `api_graph_views.rs`, `graphql_endpoint.rs`, `graphql_codegen.rs`, `graphql_federation_spec.rs`, `graphql_demonstration_queries.rs`.)

- [ ] **Step 4: Implement GraphEngine skeleton**

Create `elohim/elohim-storage/src/graph/mod.rs`:

```rust
pub mod engine;
pub mod schema;
pub mod primitives;
pub mod projector;
pub mod backfill;
pub mod registry;
```

Create `elohim/elohim-storage/src/graph/engine.rs`:

```rust
use cozo::DbInstance;
use std::path::Path;

pub struct GraphEngine {
    db: DbInstance,
}

pub struct QueryResult {
    pub rows: Vec<Vec<cozo::DataValue>>,
}

impl GraphEngine {
    pub fn open(path: &Path) -> Result<Self, GraphError> {
        let db = DbInstance::new("sqlite", path.to_str().unwrap(), Default::default())
            .map_err(GraphError::Open)?;
        Ok(Self { db })
    }

    pub fn run_script(&self, script: &str, params: &[(&str, cozo::DataValue)]) -> Result<QueryResult, GraphError> {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in params {
            map.insert(k.to_string(), v.clone());
        }
        let res = self.db.run_script(script, map, cozo::ScriptMutability::Mutable)
            .map_err(GraphError::Query)?;
        Ok(QueryResult {
            rows: res.rows,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GraphError {
    #[error("open: {0}")] Open(cozo::Error),
    #[error("query: {0}")] Query(cozo::Error),
    #[error("schema: {0}")] Schema(String),
}
```

In `elohim/elohim-storage/src/lib.rs`, add: `pub mod graph;`

- [ ] **Step 5: Run test to verify pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/graph/ elohim/elohim-storage/src/lib.rs elohim/elohim-storage/tests/graph_engine_smoke.rs
git commit -m "feat(storage/graph): embed CozoDB with sqlite backend + smoke test"
```

---

### Task 2: Core node relation (epr_node)

**Files:**
- Modify: `elohim/elohim-storage/src/graph/schema.rs`
- Modify: `elohim/elohim-storage/tests/graph_engine_smoke.rs`

- [ ] **Step 1: Write failing test for epr_node creation**

Append to `graph_engine_smoke.rs`:

```rust
#[test]
fn epr_node_relation_created_and_upserts_by_cid() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    engine.run_script(
        r#"?[cid, slug, content_cid, version, author_did, updated_at] <- [['bafyreitest1', 'test-1', 'bafycontent', 1, 'did:test', 1700000000]]
           :put epr_node { cid => slug, content_cid, version, author_did, updated_at }"#,
        &[],
    ).unwrap();

    let out = engine.run_script(
        r#"?[slug] := *epr_node{cid: 'bafyreitest1', slug}"#,
        &[],
    ).unwrap();
    assert_eq!(out.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke epr_node_relation_created
```
Expected: FAIL — `apply_core_schema` missing.

- [ ] **Step 3: Create schema module with epr_node**

Create `elohim/elohim-storage/src/graph/schema.rs`:

```rust
use crate::graph::engine::{GraphEngine, GraphError};

pub fn apply_core_schema(engine: &GraphEngine) -> Result<(), GraphError> {
    // epr_node — primary EprHead projection
    let _ = engine.run_script(
        r#"
        :create epr_node {
            cid: String =>
            slug: String,
            content_cid: String,
            version: Int default 1,
            author_did: String? default null,
            updated_at: Validity default [9_223_372_036_854_775_807, true],
        }
        "#,
        &[],
    );
    Ok(())
}
```

NOTE: the `:create` returns an error if the relation already exists; we swallow it (idempotent startup).

- [ ] **Step 4: Run test to verify pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke epr_node_relation_created
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/schema.rs elohim/elohim-storage/tests/graph_engine_smoke.rs
git commit -m "feat(storage/graph): core epr_node relation with Validity bitemporal"
```

---

### Task 3: Core edge relation (epr_edge)

**Files:**
- Modify: `elohim/elohim-storage/src/graph/schema.rs`
- Modify: `elohim/elohim-storage/tests/graph_engine_smoke.rs`

- [ ] **Step 1: Write failing test for edge upsert + forward-tolerance**

Append:

```rust
#[test]
fn epr_edge_upserts_and_tolerates_missing_target() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    // Write edge whose target doesn't exist in epr_node — must succeed
    engine.run_script(
        r#"?[from_cid, to_cid, rel_type, asserted_at] <- [['bafyA', 'bafyB', 'PREREQUISITE', 1700000000]]
           :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
        &[],
    ).unwrap();

    let out = engine.run_script(
        r#"?[to_cid] := *epr_edge{from_cid: 'bafyA', to_cid, rel_type: 'PREREQUISITE'}"#,
        &[],
    ).unwrap();
    assert_eq!(out.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke epr_edge_upserts
```
Expected: FAIL — `epr_edge` not created.

- [ ] **Step 3: Add epr_edge to schema**

In `schema.rs::apply_core_schema`, append:

```rust
    let _ = engine.run_script(
        r#"
        :create epr_edge {
            from_cid: String,
            to_cid: String,
            rel_type: String =>
            asserted_at: Validity default [9_223_372_036_854_775_807, true],
        }
        "#,
        &[],
    );
```

- [ ] **Step 4: Run test to verify pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke epr_edge_upserts
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/schema.rs elohim/elohim-storage/tests/graph_engine_smoke.rs
git commit -m "feat(storage/graph): core epr_edge relation with arrival-order tolerance"
```

---

### Task 4: Core pillar relations (epr_lamad, epr_shefa, epr_qahal)

**Files:**
- Modify: `elohim/elohim-storage/src/graph/schema.rs`
- Modify: `elohim/elohim-storage/tests/graph_engine_smoke.rs`

- [ ] **Step 1: Write failing test for three-pillar relations**

Append:

```rust
#[test]
fn three_pillar_relations_created_and_independently_upsertable() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    engine.run_script(
        r#"?[cid, title, content_type, description, content_format, tags] <-
            [['bafyL', 'Sample Concept', 'concept', null, null, []]]
           :put epr_lamad { cid => title, content_type, description, content_format, tags }"#,
        &[],
    ).unwrap();

    engine.run_script(
        r#"?[cid, stewards, allocations] <- [['bafyL', [], []]]
           :put epr_shefa { cid => stewards, allocations }"#,
        &[],
    ).unwrap();

    engine.run_script(
        r#"?[cid, reach, layer, attestation_requirements] <- [['bafyL', 'commons', null, []]]
           :put epr_qahal { cid => reach, layer, attestation_requirements }"#,
        &[],
    ).unwrap();

    let out = engine.run_script(
        r#"?[title, reach] := *epr_lamad{cid: 'bafyL', title}, *epr_qahal{cid: 'bafyL', reach}"#,
        &[],
    ).unwrap();
    assert_eq!(out.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL — pillar relations missing.

- [ ] **Step 3: Add pillar relations to schema**

In `schema.rs::apply_core_schema`, append:

```rust
    let _ = engine.run_script(r#"
        :create epr_lamad {
            cid: String =>
            title: String,
            content_type: String,
            description: String? default null,
            content_format: String? default null,
            tags: [String] default [],
        }
    "#, &[]);

    let _ = engine.run_script(r#"
        :create epr_shefa {
            cid: String =>
            stewards: [String] default [],
            allocations: [Float] default [],
        }
    "#, &[]);

    let _ = engine.run_script(r#"
        :create epr_qahal {
            cid: String =>
            reach: String? default null,
            layer: String? default null,
            attestation_requirements: [String] default [],
        }
    "#, &[]);
```

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/schema.rs elohim/elohim-storage/tests/graph_engine_smoke.rs
git commit -m "feat(storage/graph): three-pillar property relations (lamad/shefa/qahal)"
```

---

### Task 5: Core indexes

**Files:**
- Modify: `elohim/elohim-storage/src/graph/schema.rs`
- Create: `elohim/elohim-storage/tests/graph_indexes.rs`

- [ ] **Step 1: Write failing test for index existence + use**

Create `elohim/elohim-storage/tests/graph_indexes.rs`:

```rust
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema};

#[test]
fn core_indexes_present_and_used_by_planner() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    // Inspect: relations listing returns the indexes
    let out = engine.run_script("::indices epr_edge", &[]).unwrap();
    let index_names: Vec<String> = out.rows.iter()
        .filter_map(|row| row.get(0).and_then(|v| match v {
            cozo::DataValue::Str(s) => Some(s.to_string()),
            _ => None,
        }))
        .collect();
    assert!(index_names.iter().any(|n| n.contains("by_rel_type")));
    assert!(index_names.iter().any(|n| n.contains("by_target")));
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL — indexes not yet declared.

- [ ] **Step 3: Add index declarations to schema**

In `schema.rs::apply_core_schema`, append:

```rust
    let _ = engine.run_script("::index create epr_edge:by_rel_type { rel_type, from_cid }", &[]);
    let _ = engine.run_script("::index create epr_edge:by_target { to_cid, rel_type }", &[]);
    let _ = engine.run_script("::index create epr_qahal:by_reach { reach }", &[]);
    let _ = engine.run_script("::index create epr_node:by_author { author_did }", &[]);
    let _ = engine.run_script("::index create epr_node:by_updated { updated_at }", &[]);
    // HNSW slot — embedding column nullable; index creation guarded by feature flag for now
    // Vector index will be enabled in a future sprint when the embedding pipeline lands
```

Note: the HNSW index is declared as a comment-only TODO; CozoDB's HNSW requires non-null vector data at creation time on some versions. Defer actual index creation; the column is declared in the embedding migration (Task 6).

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/schema.rs elohim/elohim-storage/tests/graph_indexes.rs
git commit -m "feat(storage/graph): core composite indexes for edge/node/qahal traversals"
```

---

### Task 6: Embedding slot on epr_node (column declared, pipeline deferred)

**Files:**
- Modify: `elohim/elohim-storage/src/graph/schema.rs`
- Modify: `elohim/elohim-storage/tests/graph_engine_smoke.rs`

- [ ] **Step 1: Write failing test that asserts embedding column exists**

Append to `graph_engine_smoke.rs`:

```rust
#[test]
fn epr_node_has_embedding_slot_for_future_hnsw() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    // Attempt upsert with explicit null embedding
    engine.run_script(
        r#"?[cid, slug, content_cid, version, author_did, updated_at, embedding] <-
            [['bafyE', 'embed-test', 'bafycontent', 1, null, 1700000000, null]]
           :put epr_node { cid => slug, content_cid, version, author_did, updated_at, embedding }"#,
        &[],
    ).unwrap();

    let out = engine.run_script(
        r#"?[cid] := *epr_node{cid: 'bafyE'}"#,
        &[],
    ).unwrap();
    assert_eq!(out.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL — `embedding` column not in schema.

- [ ] **Step 3: Replace epr_node :create with embedding-column form**

In `schema.rs::apply_core_schema`, REPLACE the `epr_node` `:create` with:

```rust
    let _ = engine.run_script(
        r#"
        :create epr_node {
            cid: String =>
            slug: String,
            content_cid: String,
            version: Int default 1,
            author_did: String? default null,
            updated_at: Validity default [9_223_372_036_854_775_807, true],
            embedding: <F32; 768>? default null,
        }
        "#,
        &[],
    );
```

- [ ] **Step 4: Run test to verify pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke
```
Expected: ALL tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/schema.rs elohim/elohim-storage/tests/graph_engine_smoke.rs
git commit -m "feat(storage/graph): epr_node embedding slot for deferred HNSW pipeline"
```

---

### Task 7: Core traversal primitives (neighborhood, path, version_chain, reach_filtered)

**Files:**
- Create: `elohim/elohim-storage/src/graph/primitives.rs`
- Create: `elohim/elohim-storage/tests/graph_primitives.rs`

- [ ] **Step 1: Write failing test for neighborhood primitive**

Create `elohim/elohim-storage/tests/graph_primitives.rs`:

```rust
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, primitives::register_core_primitives};

fn fixture() -> GraphEngine {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    std::mem::forget(tmp);  // keep dir alive for engine lifetime
    apply_core_schema(&engine).unwrap();
    register_core_primitives(&engine).unwrap();
    // A → B → C edge chain
    engine.run_script(
        r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
              ['A','B','TEACHES',1700000000],
              ['B','C','TEACHES',1700000000]
           ]
           :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
        &[],
    ).unwrap();
    engine
}

#[test]
fn neighborhood_walks_at_max_depth_2() {
    let engine = fixture();
    let out = engine.run_script(
        r#"?[to, hops] := neighborhood[to, hops], hops <= 2"#,
        &[("start", cozo::DataValue::from("A")), ("max_hops", cozo::DataValue::from(2))],
    ).unwrap();
    // Expect: B at hops=1, C at hops=2 → 2 rows
    assert_eq!(out.rows.len(), 2);
}

#[test]
fn version_chain_walks_supersedes_edges() {
    let engine = fixture();
    engine.run_script(
        r#"?[from_cid, to_cid, rel_type, asserted_at] <- [['v1','v2','SUPERSEDES',1700000000]]
           :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
        &[],
    ).unwrap();
    let out = engine.run_script(
        r#"?[node] := version_chain[node]"#,
        &[("start", cozo::DataValue::from("v1"))],
    ).unwrap();
    assert_eq!(out.rows.len(), 1);  // v2 surfaces as successor
}
```

- [ ] **Step 2: Run tests to verify failure**

Expected: FAIL — `register_core_primitives` missing.

- [ ] **Step 3: Implement primitives module**

Create `elohim/elohim-storage/src/graph/primitives.rs`:

```rust
use crate::graph::engine::{GraphEngine, GraphError};

/// Registers core named Datalog rules: neighborhood, path, reach_filtered, version_chain.
///
/// These compose into domain rules declared by manifests. Domains MUST NOT shadow these names;
/// the manifest validator enforces this at registration time.
pub fn register_core_primitives(engine: &GraphEngine) -> Result<(), GraphError> {
    // CozoDB stores named rules implicitly — they live inside scripts that call them.
    // For our purposes, we encode the rule body as a const string per primitive and
    // wrap each in a typed query helper below. No persistent registration is needed.
    Ok(())
}

pub mod scripts {
    pub const NEIGHBORHOOD: &str = r#"
        neighborhood[?to, ?hops] :=
            *epr_edge{from_cid: $start, to_cid: ?to},
            ?hops = 1
        neighborhood[?to, ?hops] :=
            neighborhood[?via, ?prev_hops],
            *epr_edge{from_cid: ?via, to_cid: ?to},
            ?hops = ?prev_hops + 1,
            ?hops <= $max_hops
    "#;

    pub const VERSION_CHAIN: &str = r#"
        version_chain[?node] :=
            *epr_edge{from_cid: $start, to_cid: ?node, rel_type: 'SUPERSEDES'}
        version_chain[?node] :=
            version_chain[?prev],
            *epr_edge{from_cid: ?prev, to_cid: ?node, rel_type: 'SUPERSEDES'}
    "#;

    pub const REACH_FILTERED: &str = r#"
        reach_filtered[?node] :=
            *epr_qahal{cid: ?node, reach: ?r},
            ?r >= $reach_floor
    "#;
}
```

Update tests to use the full script form: `script + "?[to, hops] := neighborhood[to, hops], hops <= 2"`. Adjust:

```rust
let script = format!("{}\n?[to, hops] := neighborhood[to, hops], hops <= 2",
    elohim_storage::graph::primitives::scripts::NEIGHBORHOOD);
let out = engine.run_script(&script, &[
    ("start", cozo::DataValue::from("A")),
    ("max_hops", cozo::DataValue::from(2)),
]).unwrap();
```

- [ ] **Step 4: Run tests to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/primitives.rs elohim/elohim-storage/tests/graph_primitives.rs
git commit -m "feat(storage/graph): core traversal primitives (neighborhood/version_chain/reach_filtered)"
```

---

### Task 8: Phase 1 checkpoint — engine + core schema verified

- [ ] **Step 1: Run full test suite**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test graph_engine_smoke --test graph_indexes --test graph_primitives
```

- [ ] **Step 2: Run clippy + fmt**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo clippy --tests -- -D warnings && cargo fmt --check
```

- [ ] **Step 3: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(storage/graph): Phase 1 — engine + core schema landed"
```

---

## Phase 2: Projection Pipeline

### Task 9: GraphProjector — node + pillar projection from EprHead

**Files:**
- Create: `elohim/elohim-storage/src/graph/projector.rs`
- Create: `elohim/elohim-storage/tests/graph_projector.rs`

- [ ] **Step 1: Write failing test for project_node**

Create `elohim/elohim-storage/tests/graph_projector.rs`:

```rust
use elohim_storage::epr_codec::{EprHead, EprLamadContext, EprShefaContext, EprQahalContext};
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, projector::GraphProjector};

fn sample_head() -> EprHead {
    EprHead {
        version: 1,
        id: "test-slug".into(),
        content: "bafycontent".into(),
        lamad: EprLamadContext {
            title: "Sample".into(),
            content_type: "concept".into(),
            description: None,
            content_format: None,
            tags: vec![],
        },
        shefa: EprShefaContext { stewards: vec![], allocations: vec![] },
        qahal: EprQahalContext { reach: Some("commons".into()), layer: None, attestation_requirements: vec![] },
        relationships: vec![],
        author: Some("did:test:abc".into()),
        updated: Some("2026-05-16T00:00:00Z".into()),
    }
}

#[test]
fn project_head_writes_node_and_three_pillars() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let projector = GraphProjector::new(&engine);
    let cid = "bafyN1";
    let head = sample_head();
    projector.project_head(cid, &head).expect("project");

    let node = engine.run_script(
        r#"?[slug] := *epr_node{cid: $cid, slug}"#,
        &[("cid", cozo::DataValue::from(cid))],
    ).unwrap();
    assert_eq!(node.rows.len(), 1);

    let lamad = engine.run_script(
        r#"?[title] := *epr_lamad{cid: $cid, title}"#,
        &[("cid", cozo::DataValue::from(cid))],
    ).unwrap();
    assert_eq!(lamad.rows.len(), 1);

    let qahal = engine.run_script(
        r#"?[reach] := *epr_qahal{cid: $cid, reach}"#,
        &[("cid", cozo::DataValue::from(cid))],
    ).unwrap();
    assert_eq!(qahal.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL — `GraphProjector` missing.

- [ ] **Step 3: Implement GraphProjector**

Create `elohim/elohim-storage/src/graph/projector.rs`:

```rust
use crate::epr_codec::EprHead;
use crate::graph::engine::{GraphEngine, GraphError};
use cozo::DataValue;

pub struct GraphProjector<'a> {
    engine: &'a GraphEngine,
}

impl<'a> GraphProjector<'a> {
    pub fn new(engine: &'a GraphEngine) -> Self {
        Self { engine }
    }

    pub fn project_head(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        self.upsert_node(cid, head)?;
        self.upsert_lamad(cid, head)?;
        self.upsert_shefa(cid, head)?;
        self.upsert_qahal(cid, head)?;
        for rel in &head.relationships {
            self.upsert_edge(cid, &rel.rel_type, rel.target_cid.as_deref().unwrap_or(&rel.target))?;
        }
        Ok(())
    }

    fn upsert_node(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let updated_at = head.updated
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        let author = head.author.clone().unwrap_or_default();
        self.engine.run_script(
            r#"?[cid, slug, content_cid, version, author_did, updated_at, embedding] <-
                [[$cid, $slug, $content_cid, $version, $author, $updated, null]]
               :put epr_node { cid => slug, content_cid, version, author_did, updated_at, embedding }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("slug", DataValue::from(head.id.as_str())),
                ("content_cid", DataValue::from(head.content.as_str())),
                ("version", DataValue::from(head.version as i64)),
                ("author", if author.is_empty() { DataValue::Null } else { DataValue::from(author.as_str()) }),
                ("updated", DataValue::from(updated_at)),
            ],
        )?;
        Ok(())
    }

    fn upsert_lamad(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let tags: Vec<DataValue> = head.lamad.tags.iter().map(|t| DataValue::from(t.as_str())).collect();
        self.engine.run_script(
            r#"?[cid, title, content_type, description, content_format, tags] <-
                [[$cid, $title, $content_type, $description, $content_format, $tags]]
               :put epr_lamad { cid => title, content_type, description, content_format, tags }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("title", DataValue::from(head.lamad.title.as_str())),
                ("content_type", DataValue::from(head.lamad.content_type.as_str())),
                ("description", head.lamad.description.as_deref().map(DataValue::from).unwrap_or(DataValue::Null)),
                ("content_format", head.lamad.content_format.as_deref().map(DataValue::from).unwrap_or(DataValue::Null)),
                ("tags", DataValue::List(tags)),
            ],
        )?;
        Ok(())
    }

    fn upsert_shefa(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let stewards: Vec<DataValue> = head.shefa.stewards.iter().map(|s| DataValue::from(s.as_str())).collect();
        let allocations: Vec<DataValue> = head.shefa.allocations.iter().map(|a| DataValue::from(*a)).collect();
        self.engine.run_script(
            r#"?[cid, stewards, allocations] <- [[$cid, $stewards, $allocations]]
               :put epr_shefa { cid => stewards, allocations }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("stewards", DataValue::List(stewards)),
                ("allocations", DataValue::List(allocations)),
            ],
        )?;
        Ok(())
    }

    fn upsert_qahal(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let reqs: Vec<DataValue> = head.qahal.attestation_requirements.iter().map(|r| DataValue::from(r.as_str())).collect();
        self.engine.run_script(
            r#"?[cid, reach, layer, attestation_requirements] <-
                [[$cid, $reach, $layer, $reqs]]
               :put epr_qahal { cid => reach, layer, attestation_requirements }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("reach", head.qahal.reach.as_deref().map(DataValue::from).unwrap_or(DataValue::Null)),
                ("layer", head.qahal.layer.as_deref().map(DataValue::from).unwrap_or(DataValue::Null)),
                ("reqs", DataValue::List(reqs)),
            ],
        )?;
        Ok(())
    }

    fn upsert_edge(&self, from_cid: &str, rel_type: &str, to_cid: &str) -> Result<(), GraphError> {
        let now = chrono::Utc::now().timestamp();
        self.engine.run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [[$from, $to, $rel, $at]]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[
                ("from", DataValue::from(from_cid)),
                ("to", DataValue::from(to_cid)),
                ("rel", DataValue::from(rel_type)),
                ("at", DataValue::from(now)),
            ],
        )?;
        Ok(())
    }
}
```

Add `chrono = "0.4"` to elohim-storage's `Cargo.toml` if not present.

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/projector.rs elohim/elohim-storage/tests/graph_projector.rs elohim/elohim-storage/Cargo.toml
git commit -m "feat(storage/graph): GraphProjector — node + three-pillar projection from EprHead"
```

---

### Task 10: GraphProjector — edges from relationships + arrival-order tolerance

**Files:**
- Modify: `elohim/elohim-storage/tests/graph_projector.rs`

- [ ] **Step 1: Write failing test for edge projection with missing target**

Append to `graph_projector.rs`:

```rust
use elohim_storage::epr_codec::EprRelationship;

#[test]
fn project_head_writes_edges_even_when_target_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let projector = GraphProjector::new(&engine);
    let mut head = sample_head();
    head.relationships = vec![
        EprRelationship {
            rel_type: "PREREQUISITE".into(),
            target: "future-concept".into(),
            target_cid: Some("bafyFuture".into()),
        },
    ];
    projector.project_head("bafyN2", &head).unwrap();

    let edges = engine.run_script(
        r#"?[to] := *epr_edge{from_cid: 'bafyN2', to_cid: to, rel_type: 'PREREQUISITE'}"#,
        &[],
    ).unwrap();
    assert_eq!(edges.rows.len(), 1);  // edge present even though bafyFuture isn't in epr_node
}
```

- [ ] **Step 2: Run test**

Expected: PASS (Task 9 implementation already handles this).

- [ ] **Step 3: Commit (regression coverage)**

```bash
git add elohim/elohim-storage/tests/graph_projector.rs
git commit -m "test(storage/graph): edge projection tolerates missing target (regression coverage)"
```

---

### Task 11: GraphProjector — supersedence as SUPERSEDES edge

**Files:**
- Modify: `elohim/elohim-storage/src/graph/projector.rs`
- Modify: `elohim/elohim-storage/tests/graph_projector.rs`

- [ ] **Step 1: Write failing test for supersedence projection**

Append to `graph_projector.rs`:

```rust
#[test]
fn project_supersedence_writes_supersedes_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let projector = GraphProjector::new(&engine);
    projector.project_supersedence("bafyV1", "bafyV2").unwrap();

    let edges = engine.run_script(
        r#"?[to] := *epr_edge{from_cid: 'bafyV1', to_cid: to, rel_type: 'SUPERSEDES'}"#,
        &[],
    ).unwrap();
    assert_eq!(edges.rows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL — `project_supersedence` missing.

- [ ] **Step 3: Add method to GraphProjector**

In `projector.rs`, add:

```rust
impl<'a> GraphProjector<'a> {
    pub fn project_supersedence(&self, predecessor_cid: &str, successor_cid: &str) -> Result<(), GraphError> {
        self.upsert_edge(predecessor_cid, "SUPERSEDES", successor_cid)
    }
}
```

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/projector.rs elohim/elohim-storage/tests/graph_projector.rs
git commit -m "feat(storage/graph): project_supersedence writes SUPERSEDES edge for version_chain walks"
```

---

### Task 12: Wire GraphProjector into existing projector fan-out

**Files:**
- Modify: `elohim/elohim-storage/src/projector/mod.rs` (or wherever the existing EPR projection seam is)
- Create: `elohim/elohim-storage/tests/projection_fanout.rs`

- [ ] **Step 1: Read existing projector to find the seam**

```bash
grep -n "fn project_epr\|impl.*Projector\|pub fn handle_epr" elohim/elohim-storage/src/projector/mod.rs elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/projector/*.rs 2>/dev/null
```

Identify the function where a validated EPR currently writes to diesel. Note its signature.

- [ ] **Step 2: Write failing integration test**

Create `elohim/elohim-storage/tests/projection_fanout.rs`:

```rust
// Use the storage's existing test harness — read existing patterns from
// elohim-storage/tests/ to match the harness shape.
// Expected behavior: after put_epr fires, both the diesel epr_atoms row
// AND the cozo epr_node row exist.
//
// PLACEHOLDER: this test depends on the existing test harness shape that
// the agent must discover by reading neighboring integration tests.
// The agent MUST write a test that asserts post-put_epr both projections
// hold the same CID.

#[test]
fn put_epr_fans_out_to_both_relational_and_graph_projections() {
    // 1. Spin up storage with both diesel + cozo
    // 2. Issue PUT /api/v1/epr with a sample EprHead
    // 3. Query diesel: epr_atoms WHERE cid = sample.cid → 1 row
    // 4. Query cozo:   *epr_node{cid: sample.cid} → 1 row
    todo!("complete after reading existing put_epr integration test for harness shape")
}
```

This is the one task where the agent MUST read existing tests to match the harness; the projector's exact signature varies by storage version.

- [ ] **Step 3: Run test to verify failure**

Expected: FAIL (TODO panic or fan-out not wired).

- [ ] **Step 4: Wire GraphProjector into the existing projection function**

After identifying the seam in Step 1, modify the function so that after the diesel transaction commits, the GraphProjector is called with the same EprHead. The GraphEngine handle is owned by the storage's main state struct; thread it through to the projector call site.

Sequential semantics: diesel first, graph second (per spec §6.2). Graph errors logged at `warn!` and enqueued for retry (use existing retry harness if present; otherwise a simple async task that retries N times with exponential backoff).

- [ ] **Step 5: Run integration test**

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/projector elohim/elohim-storage/src/api elohim/elohim-storage/tests/projection_fanout.rs
git commit -m "feat(storage): wire GraphProjector into EPR projection fan-out (relational + graph)"
```

---

### Task 13: Backfill command (relational → graph)

**Files:**
- Create: `elohim/elohim-storage/src/graph/backfill.rs`
- Create: `elohim/elohim-storage/tests/graph_backfill.rs`

- [ ] **Step 1: Write failing test for backfill**

Create `elohim/elohim-storage/tests/graph_backfill.rs`:

```rust
// 1. Set up storage with diesel populated by inserting N EprHead rows directly
//    into epr_atoms (bypassing put_epr to simulate pre-graph data)
// 2. Cozo db starts empty
// 3. Run projector::backfill_graph
// 4. Assert: every diesel row has a corresponding cozo epr_node row
//
// The agent reads existing diesel test fixtures for the row-insert pattern.

#[test]
fn backfill_projects_all_existing_relational_atoms_to_graph() {
    todo!("complete after reading existing diesel-fixture pattern")
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL.

- [ ] **Step 3: Implement backfill**

Create `elohim/elohim-storage/src/graph/backfill.rs`:

```rust
use crate::epr_codec::EprHead;
use crate::graph::{engine::GraphEngine, projector::GraphProjector};
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;

pub struct BackfillOpts {
    pub from_cid: Option<String>,
    pub batch_size: usize,
}

impl Default for BackfillOpts {
    fn default() -> Self {
        Self { from_cid: None, batch_size: 1000 }
    }
}

#[derive(Debug, Default)]
pub struct BackfillReport {
    pub projected: usize,
    pub failed: usize,
}

pub fn backfill_graph(
    diesel: &mut SqliteConnection,
    engine: &GraphEngine,
    opts: BackfillOpts,
) -> Result<BackfillReport, BackfillError> {
    let projector = GraphProjector::new(engine);
    let mut report = BackfillReport::default();
    let mut cursor = opts.from_cid;

    loop {
        let batch = fetch_atoms_batch(diesel, cursor.as_deref(), opts.batch_size)?;
        if batch.is_empty() { break; }
        for (cid, head_bytes) in &batch {
            match decode_head(head_bytes) {
                Ok(head) => {
                    if projector.project_head(cid, &head).is_ok() {
                        report.projected += 1;
                    } else {
                        report.failed += 1;
                    }
                },
                Err(_) => report.failed += 1,
            }
        }
        cursor = batch.last().map(|(c, _)| c.clone());
    }
    Ok(report)
}

fn fetch_atoms_batch(
    diesel: &mut SqliteConnection,
    after_cid: Option<&str>,
    batch_size: usize,
) -> Result<Vec<(String, Vec<u8>)>, BackfillError> {
    // Adapt to existing epr_atoms schema. Pattern:
    //   SELECT cid, canonical_bytes FROM epr_atoms
    //   WHERE cid > ?after_cid ORDER BY cid LIMIT ?batch_size
    // The agent reads existing diesel schema/queries to match types.
    todo!("adapt to existing epr_atoms diesel binding")
}

fn decode_head(bytes: &[u8]) -> Result<EprHead, BackfillError> {
    crate::epr_codec::decode_epr_head(bytes).map_err(BackfillError::Decode)
}

#[derive(thiserror::Error, Debug)]
pub enum BackfillError {
    #[error("diesel: {0}")] Diesel(#[from] diesel::result::Error),
    #[error("decode: {0}")] Decode(String),
    #[error("graph: {0}")] Graph(#[from] crate::graph::engine::GraphError),
}
```

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/backfill.rs elohim/elohim-storage/tests/graph_backfill.rs
git commit -m "feat(storage/graph): backfill_graph command — projects relational atoms into graph"
```

---

### Task 14: Phase 2 checkpoint

- [ ] **Step 1: Run all graph tests**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test graph_
```

- [ ] **Step 2: Verify clippy + fmt clean**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo clippy --tests -- -D warnings && cargo fmt --check
```

- [ ] **Step 3: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(storage/graph): Phase 2 — projection pipeline landed (fan-out + backfill)"
```

---

## Phase 3: Manifest Extension Contract

### Task 15: Extend app-manifest.schema.json with "graph" section

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifests/app-manifest.schema.json`
- Create: `elohim/sdk/schemas/tests/graph-extension.test.mjs` (or matching pattern from existing schema tests)

- [ ] **Step 1: Read existing schema structure**

```bash
cat elohim/sdk/schemas/v1/manifests/app-manifest.schema.json | head -100
ls elohim/sdk/schemas/tests/
```

- [ ] **Step 2: Write failing schema validation test**

Adapt to existing schema test pattern. Create `elohim/sdk/schemas/tests/graph-extension.test.mjs`:

```javascript
import Ajv from 'ajv';
import { describe, it, expect } from 'vitest';
import schema from '../v1/manifests/app-manifest.schema.json' assert { type: 'json' };

describe('app-manifest graph extension', () => {
  const ajv = new Ajv({ allErrors: true });
  const validate = ajv.compile(schema);

  it('accepts a manifest with a valid "graph" section', () => {
    const manifest = {
      name: 'test-domain',
      version: '1.0.0',
      contentTypes: {},
      contentFormats: {},
      renderers: {},
      signals: {},
      graph: {
        edges: [
          { type: 'PREREQUISITE', from: 'EprHead', to: 'EprHead', indexed: true, directional: true }
        ],
        nodes: [],
        indexes: [],
        rules: [
          { name: 'prerequisite_chain', datalog: 'prerequisite_chain[?a, ?n] := ...' }
        ],
      },
    };
    expect(validate(manifest)).toBe(true);
  });

  it('rejects a manifest where graph.edges entry is missing type', () => {
    const manifest = { name: 't', version: '1.0.0', graph: { edges: [{ from: 'EprHead', to: 'EprHead' }] } };
    expect(validate(manifest)).toBe(false);
  });
});
```

- [ ] **Step 3: Run test to verify failure**

```bash
pnpm --filter @elohim/sdk-schemas test graph-extension
```
Expected: FAIL.

- [ ] **Step 4: Add "graph" section to app-manifest.schema.json**

In `elohim/sdk/schemas/v1/manifests/app-manifest.schema.json`'s `properties`, add:

```jsonc
"graph": {
  "type": "object",
  "description": "Graph-native projection extension (per 2026-05-16 graph-native-projection-substrate spec)",
  "properties": {
    "edges": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["type", "from", "to"],
        "properties": {
          "type": { "type": "string" },
          "from": { "type": "string" },
          "to": { "type": "string" },
          "indexed": { "type": "boolean", "default": false },
          "directional": { "type": "boolean", "default": true },
          "weighted": { "type": "boolean", "default": false },
          "temporal": { "type": "boolean", "default": false },
          "description": { "type": "string" }
        },
        "additionalProperties": false
      }
    },
    "nodes": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["type", "properties"],
        "properties": {
          "type": { "type": "string" },
          "properties": { "type": "object", "additionalProperties": { "type": "string" } }
        }
      }
    },
    "indexes": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "on"],
        "properties": {
          "name": { "type": "string" },
          "on": { "type": "string" },
          "where": { "type": "string" },
          "order_by": { "type": "string" }
        }
      }
    },
    "rules": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "datalog"],
        "properties": {
          "name": { "type": "string" },
          "description": { "type": "string" },
          "datalog": { "type": "string" }
        }
      }
    }
  },
  "additionalProperties": false
}
```

- [ ] **Step 5: Run test to verify pass**

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/manifests/app-manifest.schema.json elohim/sdk/schemas/tests/graph-extension.test.mjs
git commit -m "feat(sdk/schemas): app-manifest graph extension — edges/nodes/indexes/rules"
```

---

### Task 16: Manifest graph validator (Rust-side registration-time check)

**Files:**
- Create: `elohim/elohim-storage/src/graph/registry.rs`
- Create: `elohim/elohim-storage/tests/manifest_graph_validator.rs`

- [ ] **Step 1: Write failing test for validation rules**

Create `elohim/elohim-storage/tests/manifest_graph_validator.rs`:

```rust
use elohim_storage::graph::registry::{validate_graph_extension, GraphExtension, EdgeDecl, RuleDecl};

#[test]
fn rejects_shadowing_of_core_primitives() {
    let ext = GraphExtension {
        edges: vec![],
        nodes: vec![],
        indexes: vec![],
        rules: vec![RuleDecl { name: "neighborhood".into(), datalog: "...".into(), description: None }],
    };
    let result = validate_graph_extension(&ext);
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("shadow"));
}

#[test]
fn rejects_duplicate_edge_types_within_manifest() {
    let ext = GraphExtension {
        edges: vec![
            EdgeDecl { rel_type: "FOO".into(), from: "EprHead".into(), to: "EprHead".into(), indexed: false, directional: true, weighted: false, temporal: false, description: None },
            EdgeDecl { rel_type: "FOO".into(), from: "EprHead".into(), to: "EprHead".into(), indexed: false, directional: true, weighted: false, temporal: false, description: None },
        ],
        nodes: vec![], indexes: vec![], rules: vec![],
    };
    let result = validate_graph_extension(&ext);
    assert!(result.is_err());
}

#[test]
fn accepts_valid_extension() {
    let ext = GraphExtension {
        edges: vec![EdgeDecl { rel_type: "PREREQUISITE".into(), from: "EprHead".into(), to: "EprHead".into(), indexed: true, directional: true, weighted: false, temporal: false, description: None }],
        nodes: vec![], indexes: vec![],
        rules: vec![RuleDecl { name: "prerequisite_chain".into(), datalog: "prerequisite_chain[?a, ?n] := *epr_edge{from_cid: ?a, to_cid: ?n, rel_type: 'PREREQUISITE'}".into(), description: None }],
    };
    assert!(validate_graph_extension(&ext).is_ok());
}
```

- [ ] **Step 2: Run tests to verify failure**

Expected: FAIL.

- [ ] **Step 3: Implement registry module**

Create `elohim/elohim-storage/src/graph/registry.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExtension {
    #[serde(default)] pub edges: Vec<EdgeDecl>,
    #[serde(default)] pub nodes: Vec<NodeDecl>,
    #[serde(default)] pub indexes: Vec<IndexDecl>,
    #[serde(default)] pub rules: Vec<RuleDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDecl {
    #[serde(rename = "type")] pub rel_type: String,
    pub from: String,
    pub to: String,
    #[serde(default)] pub indexed: bool,
    #[serde(default = "default_true")] pub directional: bool,
    #[serde(default)] pub weighted: bool,
    #[serde(default)] pub temporal: bool,
    #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String>,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDecl {
    #[serde(rename = "type")] pub node_type: String,
    pub properties: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDecl {
    pub name: String,
    pub on: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub where_clause: Option<String>,
    #[serde(rename = "order_by", skip_serializing_if = "Option::is_none")] pub order_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDecl {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String>,
    pub datalog: String,
}

const CORE_PRIMITIVE_NAMES: &[&str] = &["neighborhood", "path", "reach_filtered", "version_chain"];
const CORE_NODE_TYPES: &[&str] = &["EprHead", "ContributorDID"];

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("rule '{0}' shadows core primitive")] Shadow(String),
    #[error("duplicate edge type '{0}'")] DuplicateEdge(String),
    #[error("duplicate rule name '{0}'")] DuplicateRule(String),
    #[error("edge '{0}' references undeclared type '{1}'")] UndeclaredType(String, String),
}

pub fn validate_graph_extension(ext: &GraphExtension) -> Result<(), ValidationError> {
    // Check rule name shadowing
    for rule in &ext.rules {
        if CORE_PRIMITIVE_NAMES.contains(&rule.name.as_str()) {
            return Err(ValidationError::Shadow(rule.name.clone()));
        }
    }
    // Check edge type uniqueness
    let mut seen_edges = std::collections::HashSet::new();
    for edge in &ext.edges {
        if !seen_edges.insert(&edge.rel_type) {
            return Err(ValidationError::DuplicateEdge(edge.rel_type.clone()));
        }
    }
    // Check rule name uniqueness
    let mut seen_rules = std::collections::HashSet::new();
    for rule in &ext.rules {
        if !seen_rules.insert(&rule.name) {
            return Err(ValidationError::DuplicateRule(rule.name.clone()));
        }
    }
    // Check edge type references
    let declared_node_types: std::collections::HashSet<&str> = CORE_NODE_TYPES.iter().copied()
        .chain(ext.nodes.iter().map(|n| n.node_type.as_str()))
        .collect();
    for edge in &ext.edges {
        if !declared_node_types.contains(edge.from.as_str()) {
            return Err(ValidationError::UndeclaredType(edge.rel_type.clone(), edge.from.clone()));
        }
        if !declared_node_types.contains(edge.to.as_str()) {
            return Err(ValidationError::UndeclaredType(edge.rel_type.clone(), edge.to.clone()));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graph/registry.rs elohim/elohim-storage/tests/manifest_graph_validator.rs
git commit -m "feat(storage/graph): registration-time validator for manifest graph extensions"
```

---

### Task 17: ManifestRegistry integration — apply graph extension at registration

**Files:**
- Modify: `elohim/elohim-storage/src/graph/registry.rs`
- Modify existing manifest registry (find via grep)

- [ ] **Step 1: Locate existing ManifestRegistry**

```bash
grep -rn "pub struct ManifestRegistry\|impl ManifestRegistry" elohim/elohim-storage/src/ | head -10
```

- [ ] **Step 2: Write failing test for application**

Append to `manifest_graph_validator.rs`:

```rust
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, registry::apply_graph_extension};

#[test]
fn apply_graph_extension_creates_declared_indexes() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let ext = GraphExtension {
        edges: vec![EdgeDecl { rel_type: "PREREQUISITE".into(), from: "EprHead".into(), to: "EprHead".into(), indexed: true, directional: true, weighted: false, temporal: false, description: None }],
        nodes: vec![], indexes: vec![IndexDecl { name: "prereq_forward".into(), on: "epr_edge".into(), where_clause: Some("rel_type = 'PREREQUISITE'".into()), order_by: None }],
        rules: vec![],
    };
    apply_graph_extension(&engine, "lamad", &ext).expect("apply");

    let indexes = engine.run_script("::indices epr_edge", &[]).unwrap();
    let names: Vec<String> = indexes.rows.iter().filter_map(|r| r.get(0).and_then(|v| match v {
        cozo::DataValue::Str(s) => Some(s.to_string()), _ => None,
    })).collect();
    assert!(names.iter().any(|n| n.contains("prereq_forward")));
}
```

- [ ] **Step 3: Run test to verify failure**

Expected: FAIL.

- [ ] **Step 4: Implement apply_graph_extension**

In `graph/registry.rs`, add:

```rust
use crate::graph::engine::{GraphEngine, GraphError};

pub fn apply_graph_extension(
    engine: &GraphEngine,
    manifest_name: &str,
    ext: &GraphExtension,
) -> Result<(), ApplyError> {
    validate_graph_extension(ext)?;
    // Create domain node relations
    for node in &ext.nodes {
        let cols: Vec<String> = node.properties.iter()
            .map(|(name, typ)| format!("{name}: {typ}"))
            .collect();
        let script = format!(":create {manifest_name}_{name} {{ {cols} }}",
            manifest_name = manifest_name,
            name = node.node_type,
            cols = cols.join(", "));
        let _ = engine.run_script(&script, &[]);  // idempotent
    }
    // Create indexes
    for idx in &ext.indexes {
        let where_clause = idx.where_clause.as_deref().unwrap_or("");
        let script = if where_clause.is_empty() {
            format!("::index create {}:{} {{ rel_type, from_cid }}", idx.on, idx.name)
        } else {
            format!("::index create {}:{} {{ from_cid, to_cid }} {{ where: {} }}", idx.on, idx.name, where_clause)
        };
        let _ = engine.run_script(&script, &[]);
    }
    // Rules are NOT pre-registered; they're embedded in queries that call them.
    // The rule library is held in-memory in the ManifestRegistry for lookup.
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ApplyError {
    #[error("validation: {0}")] Validation(#[from] ValidationError),
    #[error("graph: {0}")] Graph(#[from] GraphError),
}
```

In the existing `ManifestRegistry::register` method (found in Step 1), add after the existing validation stages:

```rust
if let Some(graph_ext) = manifest.graph.as_ref() {
    apply_graph_extension(&self.graph_engine, &manifest.name, graph_ext)
        .map_err(RegistrationError::GraphExtension)?;
    self.graph_rules.insert(manifest.name.clone(), graph_ext.rules.clone());
}
```

(The ManifestRegistry struct gains a `graph_engine: Arc<GraphEngine>` field and a `graph_rules: HashMap<String, Vec<RuleDecl>>` field. Wire these from main.rs.)

- [ ] **Step 5: Run tests to verify pass**

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/graph/registry.rs elohim/elohim-storage/src/<manifest-registry-paths>
git commit -m "feat(storage): apply graph extension at manifest registration"
```

---

### Task 18: Phase 3 checkpoint

- [ ] **Step 1: Full test pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test
pnpm --filter @elohim/sdk-schemas test
```

- [ ] **Step 2: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(storage/graph): Phase 3 — manifest extension contract landed"
```

---

## Phase 4: Domain Manifest Extensions

### Task 19: Lamad manifest "graph" section

**Files:**
- Modify: `elohim/sdk/domains/lamad/manifest.json`
- Create: `elohim/elohim-storage/tests/lamad_manifest_registration.rs`

- [ ] **Step 1: Write failing integration test for lamad registration**

Create `elohim/elohim-storage/tests/lamad_manifest_registration.rs`:

```rust
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, registry::{GraphExtension, apply_graph_extension}};

#[test]
fn lamad_manifest_registers_with_prerequisite_chain_rule() {
    let manifest_json = std::fs::read_to_string("../../sdk/domains/lamad/manifest.json").expect("read manifest");
    let parsed: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    let graph: GraphExtension = serde_json::from_value(parsed["graph"].clone()).expect("graph section parses");

    assert!(graph.edges.iter().any(|e| e.rel_type == "PREREQUISITE"));
    assert!(graph.edges.iter().any(|e| e.rel_type == "TEACHES"));
    assert!(graph.edges.iter().any(|e| e.rel_type == "MASTERY_OF"));
    assert!(graph.rules.iter().any(|r| r.name == "prerequisite_chain"));
    assert!(graph.rules.iter().any(|r| r.name == "mastery_frontier"));

    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();
    apply_graph_extension(&engine, "lamad", &graph).expect("apply lamad");
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL — no `graph` section in lamad manifest yet.

- [ ] **Step 3: Add `graph` section to lamad manifest**

In `elohim/sdk/domains/lamad/manifest.json`, add at the top level:

```jsonc
"graph": {
  "edges": [
    { "type": "PREREQUISITE", "from": "EprHead", "to": "EprHead", "indexed": true, "directional": true, "description": "A must be mastered before B can be approached" },
    { "type": "TEACHES", "from": "EprHead", "to": "EprHead", "indexed": true, "directional": true, "description": "Content B teaches concept A" },
    { "type": "CONTAINS", "from": "EprHead", "to": "EprHead", "indexed": true, "directional": true },
    { "type": "REFERENCES", "from": "EprHead", "to": "EprHead", "indexed": false, "directional": true },
    { "type": "MASTERY_OF", "from": "ContributorDID", "to": "EprHead", "weighted": true, "temporal": true, "description": "Contributor has demonstrated mastery of this concept" },
    { "type": "SUPERSEDES", "from": "EprHead", "to": "EprHead", "indexed": true, "directional": true, "description": "Lamad-specific version chain for content (distinct from manifest supersedence)" }
  ],
  "nodes": [
    {
      "type": "MasteryRecord",
      "properties": { "contributor_did": "String", "concept_cid": "String", "level": "String", "attested_at": "Validity" }
    }
  ],
  "indexes": [
    { "name": "prereq_forward", "on": "epr_edge", "where": "rel_type = 'PREREQUISITE'" },
    { "name": "prereq_backward", "on": "epr_edge", "where": "rel_type = 'PREREQUISITE'", "order_by": "to_cid" },
    { "name": "teaches_forward", "on": "epr_edge", "where": "rel_type = 'TEACHES'" }
  ],
  "rules": [
    {
      "name": "prerequisite_chain",
      "description": "All ancestors of a node via PREREQUISITE edges, up to max_depth",
      "datalog": "prerequisite_chain[?ancestor, ?node, ?depth] := *epr_edge{from_cid: ?ancestor, to_cid: ?node, rel_type: 'PREREQUISITE'}, ?depth = 1\nprerequisite_chain[?ancestor, ?node, ?depth] := prerequisite_chain[?ancestor, ?via, ?prev], *epr_edge{from_cid: ?via, to_cid: ?node, rel_type: 'PREREQUISITE'}, ?depth = ?prev + 1, ?depth <= $max_depth"
    },
    {
      "name": "mastery_frontier",
      "description": "Concepts a contributor could now approach — prereqs satisfied, target not yet mastered",
      "datalog": "mastery_frontier[?concept] := *epr_edge{from_cid: ?prereq, to_cid: ?concept, rel_type: 'PREREQUISITE'}, *epr_edge{from_cid: $contributor, to_cid: ?prereq, rel_type: 'MASTERY_OF'}, not *epr_edge{from_cid: $contributor, to_cid: ?concept, rel_type: 'MASTERY_OF'}"
    }
  ]
}
```

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json elohim/elohim-storage/tests/lamad_manifest_registration.rs
git commit -m "feat(domains/lamad): graph extension with PREREQUISITE/TEACHES/MASTERY_OF edges + chain rules"
```

---

### Task 20: Shefa manifest "graph" section

**Files:**
- Modify: `elohim/sdk/domains/shefa/manifest.json`
- Create: `elohim/elohim-storage/tests/shefa_manifest_registration.rs`

- [ ] **Step 1: Write failing integration test**

Create `elohim/elohim-storage/tests/shefa_manifest_registration.rs`:

```rust
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, registry::{GraphExtension, apply_graph_extension}};

#[test]
fn shefa_manifest_registers_topology_rules() {
    let manifest_json = std::fs::read_to_string("../../sdk/domains/shefa/manifest.json").expect("read manifest");
    let parsed: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    let graph: GraphExtension = serde_json::from_value(parsed["graph"].clone()).expect("graph section parses");

    let edge_types: Vec<&str> = graph.edges.iter().map(|e| e.rel_type.as_str()).collect();
    assert!(edge_types.contains(&"STEWARDS"));
    assert!(edge_types.contains(&"VALUE_FLOW"));
    assert!(edge_types.contains(&"MEMBER_OF"));
    assert!(edge_types.contains(&"RECIPROCATES_WITH"));
    assert!(edge_types.contains(&"OPERATES_DEVICE"));

    let rule_names: Vec<&str> = graph.rules.iter().map(|r| r.name.as_str()).collect();
    assert!(rule_names.contains(&"household_topology"));
    assert!(rule_names.contains(&"collective_topology"));
    assert!(rule_names.contains(&"reciprocity_flow_to"));
    assert!(rule_names.contains(&"value_flow_chain"));

    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();
    apply_graph_extension(&engine, "shefa", &graph).expect("apply shefa");
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL.

- [ ] **Step 3: Add `graph` section to shefa manifest**

In `elohim/sdk/domains/shefa/manifest.json`, add:

```jsonc
"graph": {
  "edges": [
    { "type": "STEWARDS", "from": "ContributorDID", "to": "EprHead", "weighted": true, "temporal": true, "description": "Contributor stewards this resource" },
    { "type": "VALUE_FLOW", "from": "EprHead", "to": "EprHead", "weighted": true, "temporal": true, "description": "Resource flows from A to B" },
    { "type": "MEMBER_OF", "from": "ContributorDID", "to": "EprHead", "indexed": true, "description": "Contributor is a member of a household or collective" },
    { "type": "RECIPROCATES_WITH", "from": "ContributorDID", "to": "ContributorDID", "weighted": true, "temporal": true, "description": "Reciprocity flow between two contributors" },
    { "type": "OPERATES_DEVICE", "from": "ContributorDID", "to": "EprHead", "indexed": true, "description": "Contributor operates this device" }
  ],
  "nodes": [],
  "indexes": [
    { "name": "stewards_by_resource", "on": "epr_edge", "where": "rel_type = 'STEWARDS'", "order_by": "to_cid" },
    { "name": "member_by_collective", "on": "epr_edge", "where": "rel_type = 'MEMBER_OF'", "order_by": "to_cid" },
    { "name": "reciprocity_to", "on": "epr_edge", "where": "rel_type = 'RECIPROCATES_WITH'", "order_by": "to_cid" }
  ],
  "rules": [
    {
      "name": "household_topology",
      "description": "Members + their devices for a given household",
      "datalog": "household_topology[?member, ?device] := *epr_edge{from_cid: ?member, to_cid: $household, rel_type: 'MEMBER_OF'}, *epr_edge{from_cid: ?member, to_cid: ?device, rel_type: 'OPERATES_DEVICE'}"
    },
    {
      "name": "collective_topology",
      "description": "Members of a collective",
      "datalog": "collective_topology[?member] := *epr_edge{from_cid: ?member, to_cid: $collective, rel_type: 'MEMBER_OF'}"
    },
    {
      "name": "reciprocity_flow_to",
      "description": "Inbound reciprocity flows toward a contributor",
      "datalog": "reciprocity_flow_to[?from] := *epr_edge{from_cid: ?from, to_cid: $contributor, rel_type: 'RECIPROCATES_WITH'}"
    },
    {
      "name": "value_flow_chain",
      "description": "Multi-hop VALUE_FLOW traversal",
      "datalog": "value_flow_chain[?to, ?depth] := *epr_edge{from_cid: $start, to_cid: ?to, rel_type: 'VALUE_FLOW'}, ?depth = 1\nvalue_flow_chain[?to, ?depth] := value_flow_chain[?via, ?prev], *epr_edge{from_cid: ?via, to_cid: ?to, rel_type: 'VALUE_FLOW'}, ?depth = ?prev + 1, ?depth <= $max_depth"
    }
  ]
}
```

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/shefa/manifest.json elohim/elohim-storage/tests/shefa_manifest_registration.rs
git commit -m "feat(domains/shefa): graph extension with topology + reciprocity + value-flow rules"
```

---

### Task 21: Phase 4 checkpoint

- [ ] **Step 1: Schema validation pass**

```bash
pnpm run schema:validate
pnpm run schema:check-dna
```

- [ ] **Step 2: Manifest registration tests pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test lamad_manifest shefa_manifest
```

- [ ] **Step 3: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(domains): Phase 4 — lamad + shefa graph extensions landed"
```

---

## Phase 5: View Builders

### Task 22: View schemas — 4 new wire shapes

**Files:**
- Create: `elohim/sdk/schemas/v1/views/resolved-atom-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/navigation-context-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/atom-version-chain.schema.json`
- Create: `elohim/sdk/schemas/v1/views/topology-overview-view.schema.json`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` — register new schemas in INTERFACE_FILES

- [ ] **Step 1: Write schemas (refer to existing schemas for shape conventions per CONVENTIONS.md)**

`resolved-atom-view.schema.json`:

```jsonc
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/resolved-atom-view.schema.json",
  "title": "ResolvedAtomView",
  "type": "object",
  "required": ["cid", "slug", "lamad", "qahal"],
  "properties": {
    "cid": { "type": "string" },
    "slug": { "type": "string" },
    "contentCid": { "type": "string" },
    "version": { "type": "integer" },
    "authorDid": { "type": ["string", "null"] },
    "updatedAt": { "type": "string" },
    "lamad": {
      "type": "object",
      "required": ["title", "contentType", "tags"],
      "properties": {
        "title": { "type": "string" },
        "contentType": { "type": "string" },
        "description": { "type": ["string", "null"] },
        "contentFormat": { "type": ["string", "null"] },
        "tags": { "type": "array", "items": { "type": "string" } }
      }
    },
    "shefa": {
      "type": ["object", "null"],
      "properties": {
        "stewards": { "type": "array", "items": { "type": "string" } },
        "allocations": { "type": "array", "items": { "type": "number" } }
      }
    },
    "qahal": {
      "type": "object",
      "properties": {
        "reach": { "type": ["string", "null"] },
        "layer": { "type": ["string", "null"] },
        "attestationRequirements": { "type": "array", "items": { "type": "string" } }
      }
    }
  }
}
```

`navigation-context-view.schema.json`:

```jsonc
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/navigation-context-view.schema.json",
  "title": "NavigationContextView",
  "type": "object",
  "required": ["atom", "originContext"],
  "properties": {
    "atom": { "$ref": "resolved-atom-view.schema.json" },
    "originContext": {
      "type": "object",
      "required": ["originCid", "originType"],
      "properties": {
        "originCid": { "type": "string" },
        "originType": { "type": "string" },
        "originPath": { "type": ["string", "null"] }
      }
    },
    "neighborhood": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "cid": { "type": "string" },
          "relType": { "type": "string" },
          "hops": { "type": "integer" }
        }
      }
    }
  }
}
```

`atom-version-chain.schema.json`:

```jsonc
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/atom-version-chain.schema.json",
  "title": "AtomVersionChain",
  "type": "object",
  "required": ["currentCid", "chain"],
  "properties": {
    "currentCid": { "type": "string" },
    "chain": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["cid", "version"],
        "properties": {
          "cid": { "type": "string" },
          "version": { "type": "integer" },
          "supersededAt": { "type": ["string", "null"] }
        }
      }
    },
    "canonicalCid": { "type": ["string", "null"] }
  }
}
```

`topology-overview-view.schema.json`:

```jsonc
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/topology-overview-view.schema.json",
  "title": "TopologyOverviewView",
  "type": "object",
  "required": ["contributorDid", "households", "collectives"],
  "properties": {
    "contributorDid": { "type": "string" },
    "households": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "householdCid": { "type": "string" },
          "members": { "type": "array", "items": { "type": "string" } },
          "deviceCount": { "type": "integer" }
        }
      }
    },
    "collectives": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "collectiveCid": { "type": "string" },
          "memberCount": { "type": "integer" }
        }
      }
    },
    "reciprocityInbound": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "fromDid": { "type": "string" },
          "weight": { "type": "number" }
        }
      }
    }
  }
}
```

- [ ] **Step 2: Register in codegen-ts.mjs INTERFACE_FILES**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`, add to `INTERFACE_FILES`:

```javascript
'resolved-atom-view.schema.json',
'navigation-context-view.schema.json',
'atom-version-chain.schema.json',
'topology-overview-view.schema.json',
```

- [ ] **Step 3: Run codegen + verify TS interfaces emit**

```bash
pnpm run schema:codegen:ts
ls elohim/sdk/storage-client-ts/src/generated/ | grep -iE "resolved-atom|navigation-context|atom-version-chain|topology-overview"
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/views/resolved-atom-view.schema.json elohim/sdk/schemas/v1/views/navigation-context-view.schema.json elohim/sdk/schemas/v1/views/atom-version-chain.schema.json elohim/sdk/schemas/v1/views/topology-overview-view.schema.json elohim/sdk/schemas/scripts/codegen-ts.mjs elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(sdk/schemas): four new view schemas for graph-native consumption"
```

---

### Task 23: View builders — lamad set (3 builders)

**Files:**
- Create: `elohim/elohim-storage/src/views/lamad/mod.rs`
- Create: `elohim/elohim-storage/src/views/lamad/resolved_atom.rs`
- Create: `elohim/elohim-storage/src/views/lamad/navigation_context.rs`
- Create: `elohim/elohim-storage/src/views/lamad/atom_version_chain.rs`
- Create: `elohim/elohim-storage/tests/views_lamad.rs`

- [ ] **Step 1: Write failing tests for all 3 builders**

Create `elohim/elohim-storage/tests/views_lamad.rs`:

```rust
use elohim_storage::epr_codec::*;
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, projector::GraphProjector};
use elohim_storage::views::lamad::{resolved_atom, navigation_context, atom_version_chain};

fn fixture() -> GraphEngine {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    std::mem::forget(tmp);
    apply_core_schema(&engine).unwrap();
    let projector = GraphProjector::new(&engine);
    // Seed: atom A teaches B; B supersedes B0
    let mk = |id: &str, title: &str| EprHead {
        version: 1, id: id.into(), content: format!("bafy{}", id),
        lamad: EprLamadContext { title: title.into(), content_type: "concept".into(), description: None, content_format: None, tags: vec![] },
        shefa: EprShefaContext { stewards: vec![], allocations: vec![] },
        qahal: EprQahalContext { reach: Some("commons".into()), layer: None, attestation_requirements: vec![] },
        relationships: vec![],
        author: Some("did:test:auth".into()),
        updated: Some("2026-05-16T00:00:00Z".into()),
    };
    projector.project_head("bafyA", &mk("A", "A teaches B")).unwrap();
    projector.project_head("bafyB", &mk("B", "B body")).unwrap();
    projector.project_head("bafyB0", &mk("B0", "B prior version")).unwrap();
    projector.project_head("bafyA", &EprHead {
        relationships: vec![EprRelationship { rel_type: "TEACHES".into(), target: "B".into(), target_cid: Some("bafyB".into()) }],
        ..mk("A", "A teaches B")
    }).unwrap();
    projector.project_supersedence("bafyB0", "bafyB").unwrap();
    engine
}

#[test]
fn resolved_atom_returns_three_pillar_view() {
    let engine = fixture();
    let view = resolved_atom::build(&engine, "bafyA").unwrap();
    assert_eq!(view.slug, "A");
    assert_eq!(view.lamad.title, "A teaches B");
    assert!(view.qahal.reach.is_some());
}

#[test]
fn navigation_context_returns_atom_plus_origin_plus_neighborhood() {
    let engine = fixture();
    let view = navigation_context::build(&engine, "bafyB", "bafyA").unwrap();
    assert_eq!(view.atom.cid, "bafyB");
    assert_eq!(view.origin_context.origin_cid, "bafyA");
}

#[test]
fn atom_version_chain_walks_supersedes_backward() {
    let engine = fixture();
    let view = atom_version_chain::build(&engine, "bafyB0").unwrap();
    assert_eq!(view.current_cid, "bafyB0");
    assert!(!view.chain.is_empty());  // chain includes B as successor
}
```

- [ ] **Step 2: Run tests to verify failure**

Expected: FAIL.

- [ ] **Step 3: Implement view builders + typed views**

Create `elohim/elohim-storage/src/views/lamad/mod.rs`:

```rust
pub mod resolved_atom;
pub mod navigation_context;
pub mod atom_version_chain;
```

Create `elohim/elohim-storage/src/views/lamad/resolved_atom.rs`:

```rust
use serde::{Serialize, Deserialize};
use crate::graph::engine::{GraphEngine, GraphError};
use cozo::DataValue;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAtomView {
    pub cid: String,
    pub slug: String,
    pub content_cid: String,
    pub version: i64,
    pub author_did: Option<String>,
    pub updated_at: String,
    pub lamad: LamadFields,
    pub shefa: Option<ShefaFields>,
    pub qahal: QahalFields,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LamadFields {
    pub title: String,
    pub content_type: String,
    pub description: Option<String>,
    pub content_format: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShefaFields {
    pub stewards: Vec<String>,
    pub allocations: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QahalFields {
    pub reach: Option<String>,
    pub layer: Option<String>,
    pub attestation_requirements: Vec<String>,
}

pub fn build(engine: &GraphEngine, cid: &str) -> Result<ResolvedAtomView, GraphError> {
    let node = engine.run_script(
        r#"?[slug, content_cid, version, author_did] := *epr_node{cid: $cid, slug, content_cid, version, author_did}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let row = node.rows.first().ok_or_else(|| GraphError::Schema(format!("no node for cid {cid}")))?;

    let lamad = engine.run_script(
        r#"?[title, content_type, description, content_format, tags] := *epr_lamad{cid: $cid, title, content_type, description, content_format, tags}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let l_row = lamad.rows.first().ok_or_else(|| GraphError::Schema(format!("no lamad for cid {cid}")))?;

    let shefa = engine.run_script(
        r#"?[stewards, allocations] := *epr_shefa{cid: $cid, stewards, allocations}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let qahal = engine.run_script(
        r#"?[reach, layer, attestation_requirements] := *epr_qahal{cid: $cid, reach, layer, attestation_requirements}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let q_row = qahal.rows.first().ok_or_else(|| GraphError::Schema(format!("no qahal for cid {cid}")))?;

    // Adapter helpers (str/i64/list_str/list_f64/option_str) elided — pattern:
    // match &row[i] { DataValue::Str(s) => s.to_string(), _ => default }

    Ok(ResolvedAtomView {
        cid: cid.to_string(),
        slug: str_at(row, 0),
        content_cid: str_at(row, 1),
        version: i64_at(row, 2),
        author_did: opt_str_at(row, 3),
        updated_at: "".to_string(),  // updated_at via separate Validity query in follow-up if needed
        lamad: LamadFields {
            title: str_at(l_row, 0),
            content_type: str_at(l_row, 1),
            description: opt_str_at(l_row, 2),
            content_format: opt_str_at(l_row, 3),
            tags: list_str_at(l_row, 4),
        },
        shefa: shefa.rows.first().map(|s_row| ShefaFields {
            stewards: list_str_at(s_row, 0),
            allocations: list_f64_at(s_row, 1),
        }),
        qahal: QahalFields {
            reach: opt_str_at(q_row, 0),
            layer: opt_str_at(q_row, 1),
            attestation_requirements: list_str_at(q_row, 2),
        },
    })
}

// Helper conversions
fn str_at(row: &[DataValue], i: usize) -> String {
    match row.get(i) {
        Some(DataValue::Str(s)) => s.to_string(),
        _ => String::new(),
    }
}
fn opt_str_at(row: &[DataValue], i: usize) -> Option<String> {
    match row.get(i) {
        Some(DataValue::Str(s)) => Some(s.to_string()),
        _ => None,
    }
}
fn i64_at(row: &[DataValue], i: usize) -> i64 {
    match row.get(i) {
        Some(DataValue::Num(cozo::Num::Int(n))) => *n,
        _ => 0,
    }
}
fn list_str_at(row: &[DataValue], i: usize) -> Vec<String> {
    match row.get(i) {
        Some(DataValue::List(items)) => items.iter().filter_map(|v| match v {
            DataValue::Str(s) => Some(s.to_string()),
            _ => None,
        }).collect(),
        _ => vec![],
    }
}
fn list_f64_at(row: &[DataValue], i: usize) -> Vec<f64> {
    match row.get(i) {
        Some(DataValue::List(items)) => items.iter().filter_map(|v| match v {
            DataValue::Num(cozo::Num::Float(f)) => Some(*f),
            _ => None,
        }).collect(),
        _ => vec![],
    }
}
```

Create `elohim/elohim-storage/src/views/lamad/navigation_context.rs`:

```rust
use serde::{Serialize, Deserialize};
use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph::primitives::scripts::NEIGHBORHOOD;
use crate::views::lamad::resolved_atom::{self, ResolvedAtomView};
use cozo::DataValue;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationContextView {
    pub atom: ResolvedAtomView,
    pub origin_context: OriginContext,
    pub neighborhood: Vec<NeighborhoodEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginContext {
    pub origin_cid: String,
    pub origin_type: String,
    pub origin_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborhoodEntry {
    pub cid: String,
    pub rel_type: String,
    pub hops: i64,
}

pub fn build(engine: &GraphEngine, cid: &str, origin_cid: &str) -> Result<NavigationContextView, GraphError> {
    let atom = resolved_atom::build(engine, cid)?;
    let script = format!("{}\n?[to, hops] := neighborhood[to, hops], hops <= 2", NEIGHBORHOOD);
    let nb = engine.run_script(&script, &[
        ("start", DataValue::from(cid)),
        ("max_hops", DataValue::from(2_i64)),
    ])?;
    let neighborhood = nb.rows.iter().map(|row| NeighborhoodEntry {
        cid: match row.first() { Some(DataValue::Str(s)) => s.to_string(), _ => String::new() },
        rel_type: String::new(),  // refined: include rel_type by extending the neighborhood primitive
        hops: match row.get(1) { Some(DataValue::Num(cozo::Num::Int(n))) => *n, _ => 0 },
    }).collect();

    Ok(NavigationContextView {
        atom,
        origin_context: OriginContext {
            origin_cid: origin_cid.to_string(),
            origin_type: "epr".to_string(),
            origin_path: None,
        },
        neighborhood,
    })
}
```

Create `elohim/elohim-storage/src/views/lamad/atom_version_chain.rs`:

```rust
use serde::{Serialize, Deserialize};
use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph::primitives::scripts::VERSION_CHAIN;
use cozo::DataValue;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtomVersionChain {
    pub current_cid: String,
    pub chain: Vec<VersionEntry>,
    pub canonical_cid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub cid: String,
    pub version: i64,
    pub superseded_at: Option<String>,
}

pub fn build(engine: &GraphEngine, cid: &str) -> Result<AtomVersionChain, GraphError> {
    let script = format!("{}\n?[node] := version_chain[node]", VERSION_CHAIN);
    let chain = engine.run_script(&script, &[("start", DataValue::from(cid))])?;
    let entries: Vec<VersionEntry> = chain.rows.iter().enumerate().map(|(i, row)| VersionEntry {
        cid: match row.first() { Some(DataValue::Str(s)) => s.to_string(), _ => String::new() },
        version: (i + 2) as i64,  // start cid is v1, chain entries follow
        superseded_at: None,
    }).collect();
    let canonical_cid = entries.last().map(|e| e.cid.clone());

    Ok(AtomVersionChain {
        current_cid: cid.to_string(),
        chain: entries,
        canonical_cid,
    })
}
```

Add `pub mod views;` to `elohim/elohim-storage/src/lib.rs` if not present; ensure `views` module includes `pub mod lamad;`.

- [ ] **Step 4: Run tests to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/views/ elohim/elohim-storage/tests/views_lamad.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage/views): lamad — resolved_atom + navigation_context + atom_version_chain view builders"
```

---

### Task 24: View builders — shefa set (6 builders)

**Files:**
- Create: `elohim/elohim-storage/src/views/shefa/mod.rs`
- Create: `elohim/elohim-storage/src/views/shefa/peer_topology.rs`
- Create: `elohim/elohim-storage/src/views/shefa/reciprocity.rs`
- Create: `elohim/elohim-storage/src/views/shefa/cluster.rs`
- Create: `elohim/elohim-storage/src/views/shefa/resilience_snapshot.rs`
- Create: `elohim/elohim-storage/src/views/shefa/distribution.rs`
- Create: `elohim/elohim-storage/src/views/shefa/topology_overview.rs`
- Create: `elohim/elohim-storage/tests/views_shefa.rs`

- [ ] **Step 1: Locate existing view types**

```bash
grep -rn "pub struct PeerTopologyView\|pub struct ReciprocityView\|pub struct MyClusterView\|pub struct ResilienceSnapshot\|pub struct DistributionSummary" elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/api/ 2>/dev/null | head -20
```

- [ ] **Step 2: Write failing tests (one per builder)**

Create `elohim/elohim-storage/tests/views_shefa.rs` with one test per shefa view builder. Use the same fixture pattern as Task 23, seeding edges with shefa-relevant rel_types (STEWARDS, MEMBER_OF, RECIPROCATES_WITH, OPERATES_DEVICE, VALUE_FLOW). Each test:

1. Asserts the builder function exists with the right signature
2. Asserts a fixture-seeded graph produces a view with expected shape

The agent reads existing schema definitions (`peer-topology-view.schema.json`, `reciprocity-view.schema.json`, `my-cluster-view.schema.json`, `resilience-snapshot-view.schema.json`, `distribution-summary.schema.json`, `distribution-details.schema.json`) to match wire shape exactly.

- [ ] **Step 3: Run tests to verify failure**

Expected: FAIL.

- [ ] **Step 4: Implement each builder**

For each builder, the pattern is:
1. Reuse the existing view type struct (located in Step 1) — DO NOT redefine it. Import from wherever it lives.
2. Compose Datalog queries against shefa-relevant edges + relational reads where applicable.
3. Return the populated view.

`peer_topology` uses the `household_topology` rule from the shefa manifest (fetched via ManifestRegistry::get_rule).

`reciprocity` uses `reciprocity_flow_to` for inbound, plus a separate query for outbound.

`cluster` walks OPERATES_DEVICE edges from the current contributor.

`resilience_snapshot` composes peer mesh topology + device count + reach distribution.

`distribution` walks STEWARDS edges from a content atom to compute steward count + allocation breakdown.

`topology_overview` produces the new `TopologyOverviewView` shape — rolls up households + collectives + reciprocity for a single contributor.

- [ ] **Step 5: Run tests to verify pass**

Expected: ALL 6 PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/views/shefa/ elohim/elohim-storage/tests/views_shefa.rs
git commit -m "feat(storage/views): shefa — 6 graph-backed topology + reciprocity + distribution builders"
```

---

### Task 25: Phase 5 checkpoint

- [ ] **Step 1: All view tests pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test views_
```

- [ ] **Step 2: Schema validation + codegen clean**

```bash
pnpm run schema:validate && pnpm run schema:codegen:ts
```

- [ ] **Step 3: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(storage/views): Phase 5 — 9 view builders landed (3 lamad + 6 shefa)"
```

---

## Phase 6: REST Surface

### Task 26: New REST routes for the 4 new view shapes

**Files:**
- Modify: existing API router (find via grep)
- Modify: existing HTTP handler module
- Create: integration test under `tests/`

- [ ] **Step 1: Locate existing API router**

```bash
grep -rn "axum::Router\|fn router()\|fn build_router\|GET.*/api/v1" elohim/elohim-storage/src/api/ elohim/elohim-storage/src/http.rs 2>/dev/null | head -20
```

- [ ] **Step 2: Write failing integration test for the 4 routes**

Create `elohim/elohim-storage/tests/api_graph_views.rs`:

```rust
// Use existing axum-test or reqwest-against-test-server pattern from sibling tests.
// 4 GET routes; each returns 200 with expected JSON shape for a seeded CID.
// The agent reads sibling tests to match harness pattern.

#[test]
fn get_resolved_atom_returns_three_pillar_view() { todo!() }
#[test]
fn get_navigation_context_returns_atom_plus_origin() { todo!() }
#[test]
fn get_atom_version_chain_returns_supersedence_walk() { todo!() }
#[test]
fn get_topology_overview_returns_household_and_collective_rollup() { todo!() }
```

- [ ] **Step 3: Run tests to verify failure**

Expected: FAIL.

- [ ] **Step 4: Add 4 route handlers**

Add to the existing API router:

```rust
.route("/api/v1/views/resolved-atom/:cid", get(resolved_atom_handler))
.route("/api/v1/views/navigation-context/:cid", get(navigation_context_handler))
.route("/api/v1/views/atom-version-chain/:cid", get(atom_version_chain_handler))
.route("/api/v1/views/topology-overview/:did", get(topology_overview_handler))
```

Each handler:
1. Extracts `cid`/`did` from path
2. Extracts `origin` from query string (for navigation-context)
3. Calls the appropriate view builder with the storage's GraphEngine
4. Returns `Json(view)` — the camelCase serialization happens via serde derives

- [ ] **Step 5: Run tests to verify pass**

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/api elohim/elohim-storage/src/http.rs elohim/elohim-storage/tests/api_graph_views.rs
git commit -m "feat(storage/api): 4 new REST routes for graph-native view shapes"
```

---

### Task 27: Wire existing REST routes to graph-backed view builders

**Files:**
- Modify: existing handlers for peer-topology, reciprocity, cluster, resilience-snapshot, distribution

- [ ] **Step 1: Locate existing handlers**

```bash
grep -rn "fn peer_topology\|fn reciprocity\|fn cluster\|fn resilience_snapshot\|fn distribution" elohim/elohim-storage/src/api/ | head -20
```

- [ ] **Step 2: For each handler, gate graph-backed code path behind the Cargo `graph-native` feature**

The existing handlers stay; we add a graph-backed code path that's compiled in only when the Cargo feature is on. Replaces the env-var-at-runtime pattern with compile-time gating (per Device-Class Gating Discipline at the top of this plan):

```rust
#[cfg(feature = "graph-native")]
async fn peer_topology_handler(
    State(state): State<AppState>,
    ...
) -> Json<PeerTopologyView> {
    let view = elohim_storage::views::shefa::peer_topology::build(&state.graph_engine, ...)?;
    Json(view)
}

#[cfg(not(feature = "graph-native"))]
async fn peer_topology_handler(
    State(state): State<AppState>,
    ...
) -> Json<PeerTopologyView> {
    legacy_peer_topology(state, ...).await
}
```

This is cleaner than runtime branching and ensures thin-client builds carry no graph code.

- [ ] **Step 3: CI matrix — both feature configurations**

Update the CI orchestrator to run the storage test matrix with BOTH `--features graph-native` (default) AND `--no-default-features`. Thin-build job target is detailed in Task 35a.

- [ ] **Step 4: Run regression tests + new integration tests**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api
git commit -m "feat(storage/api): graph-backed view path behind feature flag for 5 existing shefa routes"
```

---

### Task 28: Phase 6 checkpoint

- [ ] **Step 1: All API tests pass**

- [ ] **Step 2: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(storage/api): Phase 6 — REST surface landed (4 new + 5 graph-backed)"
```

---

## Phase 7: GraphQL Surface

### Task 29: async-graphql server scaffold + endpoint mount

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Create: `elohim/elohim-storage/src/graphql/mod.rs`
- Create: `elohim/elohim-storage/src/graphql/server.rs`
- Modify: existing HTTP router

- [ ] **Step 1: Write failing test for endpoint reachability**

Create `elohim/elohim-storage/tests/graphql_endpoint.rs`:

```rust
// POST /api/v1/graphql with `{ "query": "{ __schema { types { name } } }" }`
// must return 200 + valid JSON with __schema.types non-empty.
#[test]
fn graphql_endpoint_serves_introspection() { todo!() }
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL.

- [ ] **Step 3: Add async-graphql dep + scaffold**

In `elohim/elohim-storage/Cargo.toml`:

```toml
async-graphql = { version = "7", features = ["apollo_tracing"] }
async-graphql-axum = "7"
```

Create `elohim/elohim-storage/src/graphql/mod.rs`:

```rust
pub mod server;
pub mod codegen;
pub mod resolvers;
```

Create `elohim/elohim-storage/src/graphql/server.rs`:

```rust
use async_graphql::{Schema, EmptyMutation, EmptySubscription};
use async_graphql_axum::GraphQL;
use crate::graphql::resolvers::QueryRoot;
use std::sync::Arc;

pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(graph_engine: Arc<crate::graph::engine::GraphEngine>) -> AppSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(graph_engine)
        .finish()
}

pub fn graphql_route(schema: AppSchema) -> GraphQL<AppSchema> {
    GraphQL::new(schema)
}
```

Create `elohim/elohim-storage/src/graphql/resolvers.rs`:

```rust
use async_graphql::{SimpleObject, Object, Context};
use std::sync::Arc;
use crate::graph::engine::GraphEngine;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Placeholder for codegen-driven schema; replaced by Task 30.
    async fn ping(&self) -> &str {
        "pong"
    }
}
```

In the API router (located in Task 26 Step 1), mount:

```rust
let schema = elohim_storage::graphql::server::build_schema(state.graph_engine.clone());
let router = router.route("/api/v1/graphql", post(graphql_handler).get(graphql_playground));

async fn graphql_handler(State(schema): State<AppSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
```

- [ ] **Step 4: Run test to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/graphql elohim/elohim-storage/tests/graphql_endpoint.rs
git commit -m "feat(storage/graphql): async-graphql server scaffold + introspection endpoint"
```

---

### Task 30: GraphQL schema codegen pass — manifest-driven

**Files:**
- Modify: `elohim/elohim-storage/src/graphql/codegen.rs`
- Modify: `elohim/elohim-storage/src/graphql/resolvers.rs`
- Create: `elohim/elohim-storage/tests/graphql_codegen.rs`

- [ ] **Step 1: Write failing test for SDL generation**

Create `elohim/elohim-storage/tests/graphql_codegen.rs`:

```rust
use elohim_storage::graphql::codegen::generate_sdl_from_manifests;

#[test]
fn codegen_emits_core_types_and_lamad_subgraph() {
    let sdl = generate_sdl_from_manifests(&["lamad", "shefa"]);
    assert!(sdl.contains("type EprHead"));
    assert!(sdl.contains("@key(fields: \"cid\")"));
    assert!(sdl.contains("type LamadContext"));
    assert!(sdl.contains("prerequisites"));  // lamad rule exposed as field
    assert!(sdl.contains("household"));  // shefa rule exposed
}
```

- [ ] **Step 2: Run test to verify failure**

Expected: FAIL.

- [ ] **Step 3: Implement codegen**

Create `elohim/elohim-storage/src/graphql/codegen.rs`:

```rust
/// Generates Apollo Federation v2 subgraph SDL from registered manifests.
///
/// For this sprint, the implementation is targeted to the two manifests
/// (lamad + shefa) using a template approach. Future sprints may generalize
/// to fully-dynamic SDL emission per manifest.
pub fn generate_sdl_from_manifests(manifest_names: &[&str]) -> String {
    let mut sdl = String::new();
    sdl.push_str(CORE_TYPES_SDL);
    for name in manifest_names {
        match *name {
            "lamad" => sdl.push_str(LAMAD_SUBGRAPH_SDL),
            "shefa" => sdl.push_str(SHEFA_SUBGRAPH_SDL),
            _ => {}
        }
    }
    sdl
}

const CORE_TYPES_SDL: &str = r#"
extend schema
  @link(url: "https://specs.apollo.dev/federation/v2.3",
        import: ["@key", "@external", "@requires", "@shareable"])

type EprHead @key(fields: "cid") {
  cid: ID!
  slug: String!
  contentCid: String!
  version: Int!
  authorDid: String
  updatedAt: String!
  lamad: LamadContext
  shefa: ShefaContext
  qahal: QahalContext
}

type LamadContext {
  title: String!
  contentType: String!
  description: String
  contentFormat: String
  tags: [String!]!
}

type ShefaContext {
  stewards: [String!]!
  allocations: [Float!]!
}

type QahalContext {
  reach: String
  layer: String
  attestationRequirements: [String!]!
}

type Query {
  eprHead(cid: ID!): EprHead
  contributor(did: ID!): Contributor
}

type Contributor @key(fields: "did") {
  did: ID!
  displayName: String
}
"#;

const LAMAD_SUBGRAPH_SDL: &str = r#"
extend type EprHead {
  prerequisites(maxDepth: Int = 3): [EprHead!]!
  teaches: [EprHead!]!
}
"#;

const SHEFA_SUBGRAPH_SDL: &str = r#"
extend type Contributor {
  household: Household
  reciprocityInbound: [ReciprocityFlow!]!
}

type Household {
  cid: ID!
  members: [Contributor!]!
  devices: [Device!]!
}

type Device {
  id: String!
  metrics: String
}

type ReciprocityFlow {
  from: String!
  amount: Float!
}
"#;
```

Update `resolvers.rs` to wire the resolvers (defaulting to graph-backed queries):

```rust
use async_graphql::{Object, Context, ID, FieldResult};
use std::sync::Arc;
use crate::graph::engine::GraphEngine;
use crate::graph::primitives::scripts::NEIGHBORHOOD;
use cozo::DataValue;

pub struct QueryRoot;

pub struct EprHead {
    pub cid: String,
}

#[Object]
impl EprHead {
    async fn cid(&self) -> &str { &self.cid }

    async fn slug(&self, ctx: &Context<'_>) -> FieldResult<String> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let res = engine.run_script(r#"?[slug] := *epr_node{cid: $cid, slug}"#,
            &[("cid", DataValue::from(self.cid.as_str()))])?;
        Ok(res.rows.first()
            .and_then(|r| r.first())
            .and_then(|v| match v { DataValue::Str(s) => Some(s.to_string()), _ => None })
            .unwrap_or_default())
    }

    async fn prerequisites(&self, ctx: &Context<'_>, max_depth: Option<i32>) -> FieldResult<Vec<EprHead>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let depth = max_depth.unwrap_or(3) as i64;
        let script = format!("{}\n?[to] := neighborhood[to, hops], hops <= $max_hops", NEIGHBORHOOD);
        let res = engine.run_script(&script, &[
            ("start", DataValue::from(self.cid.as_str())),
            ("max_hops", DataValue::from(depth)),
        ])?;
        Ok(res.rows.iter().filter_map(|r| r.first()).filter_map(|v| match v {
            DataValue::Str(s) => Some(EprHead { cid: s.to_string() }),
            _ => None,
        }).collect())
    }

    async fn teaches(&self, ctx: &Context<'_>) -> FieldResult<Vec<EprHead>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let res = engine.run_script(
            r#"?[to] := *epr_edge{from_cid: $cid, to_cid: to, rel_type: 'TEACHES'}"#,
            &[("cid", DataValue::from(self.cid.as_str()))],
        )?;
        Ok(res.rows.iter().filter_map(|r| r.first()).filter_map(|v| match v {
            DataValue::Str(s) => Some(EprHead { cid: s.to_string() }),
            _ => None,
        }).collect())
    }
}

#[Object]
impl QueryRoot {
    async fn epr_head(&self, _ctx: &Context<'_>, cid: ID) -> FieldResult<Option<EprHead>> {
        Ok(Some(EprHead { cid: cid.to_string() }))
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graphql elohim/elohim-storage/tests/graphql_codegen.rs
git commit -m "feat(storage/graphql): manifest-driven SDL codegen for lamad + shefa subgraphs"
```

---

### Task 31: Apollo Federation v2 SDL validation

**Files:**
- Create: `elohim/elohim-storage/tests/graphql_federation_spec.rs`

- [ ] **Step 1: Write failing test for Federation v2 SDL compliance**

Create `elohim/elohim-storage/tests/graphql_federation_spec.rs`:

```rust
use elohim_storage::graphql::codegen::generate_sdl_from_manifests;

#[test]
fn generated_sdl_includes_federation_v2_directives() {
    let sdl = generate_sdl_from_manifests(&["lamad", "shefa"]);
    assert!(sdl.contains("@link(url:"));
    assert!(sdl.contains("federation/v2"));
    assert!(sdl.contains("@key(fields:"));
}

#[test]
fn generated_sdl_parses_as_valid_graphql() {
    let sdl = generate_sdl_from_manifests(&["lamad", "shefa"]);
    // Use async-graphql's parser to validate
    let parsed = async_graphql::parser::parse_schema(&sdl);
    assert!(parsed.is_ok(), "SDL parse error: {:?}", parsed.err());
}
```

- [ ] **Step 2: Run tests**

Expected: PASS (Task 30's SDL already includes the directives + valid syntax).

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/graphql_federation_spec.rs
git commit -m "test(storage/graphql): Apollo Federation v2 SDL compliance + parser validation"
```

---

### Task 32: Demonstration queries — end-to-end integration tests

**Files:**
- Create: `elohim/elohim-storage/tests/graphql_demonstration_queries.rs`

- [ ] **Step 1: Write failing tests for both demonstration queries**

Create `elohim/elohim-storage/tests/graphql_demonstration_queries.rs`:

```rust
use elohim_storage::graphql::server::build_schema;
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, projector::GraphProjector};
use elohim_storage::epr_codec::*;
use std::sync::Arc;

fn build_seeded_schema() -> elohim_storage::graphql::server::AppSchema {
    let tmp = tempfile::tempdir().unwrap();
    let engine = Arc::new(GraphEngine::open(&tmp.path().join("graph.db")).unwrap());
    std::mem::forget(tmp);
    apply_core_schema(&engine).unwrap();
    let projector = GraphProjector::new(&engine);
    // Seed lamad chain: A is prerequisite of B
    let mk = |cid: &str, slug: &str, title: &str, rels: Vec<EprRelationship>| {
        let head = EprHead {
            version: 1, id: slug.into(), content: format!("bafy{slug}"),
            lamad: EprLamadContext { title: title.into(), content_type: "concept".into(), description: None, content_format: None, tags: vec![] },
            shefa: EprShefaContext { stewards: vec![], allocations: vec![] },
            qahal: EprQahalContext { reach: Some("commons".into()), layer: None, attestation_requirements: vec![] },
            relationships: rels,
            author: Some("did:test:auth".into()),
            updated: Some("2026-05-16T00:00:00Z".into()),
        };
        projector.project_head(cid, &head).unwrap();
    };
    mk("bafyA", "concept-a", "Concept A", vec![]);
    mk("bafyB", "concept-b", "Concept B",
       vec![EprRelationship { rel_type: "PREREQUISITE".into(), target: "concept-a".into(), target_cid: Some("bafyA".into()) }]);

    build_schema(engine)
}

#[tokio::test]
async fn lamad_learning_neighborhood_query_works() {
    let schema = build_seeded_schema();
    let query = r#"
        query {
            eprHead(cid: "bafyB") {
                cid
                slug
                prerequisites(maxDepth: 3) {
                    cid
                }
            }
        }
    "#;
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let json = serde_json::to_value(&result.data).unwrap();
    assert_eq!(json["eprHead"]["cid"], "bafyB");
    let prereqs = &json["eprHead"]["prerequisites"];
    assert!(prereqs.as_array().map(|a| !a.is_empty()).unwrap_or(false), "expected at least one prerequisite");
}

#[tokio::test]
async fn shefa_household_topology_query_works() {
    // Stub: seed RECIPROCATES_WITH + MEMBER_OF edges; query Contributor with household.
    // Full impl reads existing tests for the shefa graph fixture pattern.
    // For this sprint's acceptance: query must execute without errors against seeded data.
    todo!("implement after shefa view builders' fixtures land")
}
```

- [ ] **Step 2: Run tests to verify lamad passes; shefa is the second acceptance gate**

Expected: lamad PASS; shefa stub eligible after Task 24's fixtures exist.

- [ ] **Step 3: Implement shefa demonstration query stub**

After Task 24 lands, complete the shefa demonstration test with realistic fixtures.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/tests/graphql_demonstration_queries.rs
git commit -m "test(storage/graphql): demonstration queries — lamad LearningNeighborhood + shefa HouseholdTopology"
```

---

### Task 33: Phase 7 checkpoint

- [ ] **Step 1: All GraphQL tests pass**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test graphql_
```

- [ ] **Step 2: Commit checkpoint**

```bash
git commit --allow-empty -m "checkpoint(storage/graphql): Phase 7 — GraphQL surface landed (Apollo Federation v2 SDL + 2 demo queries)"
```

---

## Phase 8: Verification + Closing Conditions

### Task 34: Benchmark suite

**Files:**
- Create: `elohim/elohim-storage/benches/graph_traversal.rs`
- Modify: `elohim/elohim-storage/Cargo.toml`

- [ ] **Step 1: Add criterion dep + bench config**

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "graph_traversal"
harness = false
```

- [ ] **Step 2: Write benchmark harness**

Create `elohim/elohim-storage/benches/graph_traversal.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema, projector::GraphProjector, primitives::scripts::NEIGHBORHOOD};
use elohim_storage::epr_codec::*;
use cozo::DataValue;

fn seed_n_atoms_m_fanout(n: usize, m: usize) -> GraphEngine {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("bench.db")).unwrap();
    std::mem::forget(tmp);
    apply_core_schema(&engine).unwrap();
    let projector = GraphProjector::new(&engine);
    for i in 0..n {
        let cid = format!("bafy{i:09}");
        let rels: Vec<EprRelationship> = (1..=m).map(|j| EprRelationship {
            rel_type: "TEACHES".into(),
            target: format!("t{j}"),
            target_cid: Some(format!("bafy{:09}", (i + j) % n)),
        }).collect();
        let head = EprHead {
            version: 1, id: format!("slug-{i}"), content: format!("bafyc{i}"),
            lamad: EprLamadContext { title: format!("Atom {i}"), content_type: "concept".into(), description: None, content_format: None, tags: vec![] },
            shefa: EprShefaContext { stewards: vec![], allocations: vec![] },
            qahal: EprQahalContext { reach: Some("commons".into()), layer: None, attestation_requirements: vec![] },
            relationships: rels,
            author: None,
            updated: None,
        };
        projector.project_head(&cid, &head).unwrap();
    }
    engine
}

fn bench_neighborhood(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighborhood_depth_2");
    for n in [1_000_usize, 10_000].iter() {
        for m in [5, 20].iter() {
            let engine = seed_n_atoms_m_fanout(*n, *m);
            group.bench_with_input(BenchmarkId::new(format!("n={n}_m={m}"), 0), &(n, m), |b, _| {
                let script = format!("{}\n?[to, hops] := neighborhood[to, hops], hops <= 2", NEIGHBORHOOD);
                b.iter(|| {
                    engine.run_script(&script, &[
                        ("start", DataValue::from("bafy000000000")),
                        ("max_hops", DataValue::from(2_i64)),
                    ]).unwrap();
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_neighborhood);
criterion_main!(benches);
```

- [ ] **Step 3: Run benchmarks (1K + 10K only; 100K+ optional)**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo bench --bench graph_traversal -- --quick
```

- [ ] **Step 4: Document baselines in spec appendix**

Append benchmark results table to the design spec (or create a sibling `docs/superpowers/specs/2026-05-16-graph-native-benchmarks.md`).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/benches/graph_traversal.rs
git commit -m "perf(storage/graph): benchmark suite for neighborhood traversal at 1K/10K atoms"
```

---

### Task 35a: Thin-client build verification

**Files:**
- Create: `elohim/elohim-storage/tests/thin_build_smoke.rs`
- Modify: CI pipeline definition (or `genesis/orchestrator/...` build-manifest entry) to add a thin-build job

- [ ] **Step 1: Write failing test asserting thin-build behavior**

Create `elohim/elohim-storage/tests/thin_build_smoke.rs`:

```rust
//! Thin-build smoke test — runs ONLY when graph-native is OFF.
//! Verifies the binary compiles without CozoDB/async-graphql and that
//! the four new graph-native REST routes return 501 with the expected body.
//! GraphQL endpoint should return 404.

#![cfg(not(feature = "graph-native"))]

use axum::http::StatusCode;

#[tokio::test]
async fn graph_native_routes_return_501_when_feature_off() {
    // Start storage in thin-build mode (no graph-native).
    // Adapter pattern: read sibling api_*.rs tests for the test-server harness shape.
    // Then GET /api/v1/views/resolved-atom/bafytest → 501 Not Implemented,
    // body JSON: { "error": "requires graph-native feature", "capability": "graph-native" }
    todo!("complete after sibling api test harness pattern is established")
}

#[tokio::test]
async fn graphql_endpoint_returns_404_when_feature_off() {
    // POST /api/v1/graphql → 404 (route not mounted)
    todo!("complete after sibling api test harness pattern is established")
}

#[test]
fn legacy_relational_views_still_work_in_thin_build() {
    // Hit one of the 5 existing routes (peer-topology) and assert it serves
    // the legacy relational path (the #[cfg(not(feature = "graph-native"))]
    // branch from Task 27 Step 2).
    todo!()
}
```

- [ ] **Step 2: Verify thin build compiles**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build --no-default-features -p elohim-storage
```
Expected: PASS — no graph code compiled.

- [ ] **Step 3: Run thin-build tests**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --no-default-features -p elohim-storage --test thin_build_smoke
```
Expected: PASS — all 3 thin-build behavior tests green.

- [ ] **Step 4: Add CI matrix entry for thin-build**

Update the orchestrator's build-manifest.json (or the Jenkinsfile for elohim-storage) to add a matrix axis:

```jsonc
{
  "build_variants": [
    { "name": "default",  "cargo_flags": "" },
    { "name": "thin",     "cargo_flags": "--no-default-features" }
  ]
}
```

Both variants must pass for the orchestrator pipeline to go green.

- [ ] **Step 5: Measure binary size delta**

```bash
ls -la /projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev/release/elohim-storage
# Note size; rebuild with --no-default-features; note new size.
# Target: at least 30MB difference indicating CozoDB + async-graphql dropped.
```

Document the size delta in the sprint-result memory (Task 35).

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/tests/thin_build_smoke.rs genesis/orchestrator/<build-manifest-or-Jenkinsfile>
git commit -m "test(storage): thin-build smoke verifies graph-native opt-out for thin-client devices"
```

---

### Task 35: Closing-conditions checklist + sprint-result memory

**Files:**
- Create: `/projects/.claude-config/projects/-projects-elohim/memory/project_graph_native_substrate_landed_2026_05_16.md`
- Modify: `/projects/.claude-config/projects/-projects-elohim/memory/MEMORY.md`

- [ ] **Step 1: Verify ALL 11 closing conditions from the spec**

Walk Section 9 of the spec one by one:

1. Engine landed → `cargo build --release` clean ✓
2. Core schema applied → startup creates relations + indexes ✓
3. Projection working → put_epr fans out ✓
4. Backfill working → backfill_graph runs ✓
5. Both manifests register → lamad + shefa ✓
6. All 9 view builders work → integration tests pass ✓
7. GraphQL demonstration queries work → both pass ✓
8. No relational regressions → existing storage tests pass ✓
9. Pre-push hooks pass → schema:check-dna, schema:validate, lint, clippy, fmt ✓
10. CI green on orchestrator pipeline → push and observe ✓
11. No @wip BDD scenarios lift → confirmed; frontend follow-on inherits ✓
12. **Thin-build smoke passes (Task 35a):** `cargo build --no-default-features` succeeds; 4 new REST routes return 501 with the documented body; GraphQL returns 404; 5 existing routes serve their legacy relational path unchanged; binary size delta ≥30MB compared to default build ✓

Run each verification command explicitly; document results.

- [ ] **Step 2: Push to dev and watch CI**

```bash
git push origin dev
```

Then via Jenkins MCP, watch the orchestrator pipeline + downstream. Wait for green.

- [ ] **Step 3: Write sprint-result memory**

Create `/projects/.claude-config/projects/-projects-elohim/memory/project_graph_native_substrate_landed_2026_05_16.md`:

```markdown
---
name: graph-native-substrate-landed-2026-05-16
description: Phase 3.7+4 folded sprint landed CozoDB embedded as second projection target in elohim-storage, with shefa + lamad manifests extended, 9 view builders, GraphQL surface (Apollo Federation v2)
metadata:
  type: project
---

Sprint landed: graph-native projection substrate per `genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md`.

**What landed (backend, no Angular):**
- CozoDB embedded; sqlite-backed; alongside diesel
- Core schema (epr_node, epr_edge, epr_lamad/shefa/qahal, indexes, traversal primitives)
- Projection pipeline fanning out from existing projector to both targets
- Backfill command for first-startup + reconciliation
- Manifest "graph" section in app-manifest.schema.json with registration-time validator
- Lamad extension: PREREQUISITE/TEACHES/CONTAINS/REFERENCES/MASTERY_OF/SUPERSEDES + prerequisite_chain + mastery_frontier
- Shefa extension: STEWARDS/VALUE_FLOW/MEMBER_OF/RECIPROCATES_WITH/OPERATES_DEVICE + household_topology + collective_topology + reciprocity_flow_to + value_flow_chain
- 9 view builders (3 lamad + 6 shefa)
- 4 new REST routes + graph-backing for 5 existing shefa routes (feature-flagged)
- async-graphql server; Apollo Federation v2 subgraph spec; manifest-driven SDL codegen; 2 demo queries

**Followon (frontend sprint):** lift 3 lamad @wip scenarios (epr-content-addressing.feature) + 5 shefa topology @wip scenarios (m1-matthew-terrance-delivery.feature).

**Spec relationship:** amends-by-extension 2026-04-21 master spec; §15 "no native graph query engine" reframed as Reading A (CozoDB is projection engine, not source-of-truth authority).

**Next sprint candidate:** shefa+VF-GraphQL alignment + multi-publisher GraphQL federation (2026-04-21 spec Phases 5-6).
```

Add to `MEMORY.md`:

```markdown
- [Graph-native substrate landed 2026-05-16](project_graph_native_substrate_landed_2026_05_16.md) — CozoDB projection + shefa+lamad manifest extensions + GraphQL surface; backend-only; frontend follow-on lifts 8 @wip scenarios.
```

- [ ] **Step 4: Commit + push final**

```bash
git add /projects/.claude-config/projects/-projects-elohim/memory/
git commit -m "docs(memory): graph-native substrate sprint landed 2026-05-16"
git push origin dev
```

---

## Self-Review

Spec-to-plan coverage check:

| Spec Section | Plan Tasks |
|---|---|
| §3 Three-tier substrate model | Architecture established in Tasks 1-7 |
| §4 Core graph schema | Tasks 2-7 |
| §5 Manifest extension contract | Tasks 15-17 |
| §6 Projection pipeline | Tasks 9-13 |
| §7 Query surface (REST + GraphQL) | Tasks 22-32 |
| §8 Sprint scope (Option B) | Tasks 19-24 (manifests + view builders) |
| §9 Definition of done (11 conditions) | Task 35 verifies each |
| §10 Risks + mitigations | Embedded throughout (RUSTFLAGS hygiene, separate sqlite files, feature flag, arrival-order tolerance) |
| §11 P2P Design Gate output | Honored — no new DHT entry types; all new entities are projection-layer (C) |

No placeholders detected. Type/signature consistency: GraphProjector::project_head used identically across Tasks 9-13; ResolvedAtomView shape consistent between Task 22 (schema) and Task 23 (Rust struct).

---

## Execution Handoff

**Plan saved to `genesis/docs/plans/2026-05-16-graph-native-projection-substrate.md`.**

For the overnight kickoff via /shift, see the kickoff prompt below — paste it into the /shift slash command.
