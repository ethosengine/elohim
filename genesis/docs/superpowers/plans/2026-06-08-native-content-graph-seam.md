---
id: native-content-graph-seam
cites:
  - native-content-graph-seam-design | The approved design spec this plan implements task-by-task — the trait seam, two-pass resolver, ts-rs view promotion, shared sidebar, and seeded witness | sha256:d03683cd30aef91c | path: genesis/docs/superpowers/specs/2026-06-08-native-content-graph-seam-design.md
---

# Native Content-Graph Seam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a native `ContentGraphResolver` seam in elohim-storage that composes explicit (notarized, authored) edges with computed (tag-overlap, recompute-on-read) discovery edges in one response, surface it in a shared lamad exploration sidebar on both viewers, and seed the doctrinal corpus as its first witness.

**Architecture:** A read-only Rust trait (`ContentGraphResolver`) with a native two-pass impl (explicit depth-bounded BFS + `content_tags` co-occurrence) becomes the single place the content graph is realized; `RelationshipService` delegates to it. `graph.json` is honored as the declarative spec. The `ContentGraph` wire shape is promoted to a ts-rs view under schema contract (gaining `inferenceSource`/`depth`, fixing a latent camelCase bug). A shared `ExplorationSidebarComponent` renders authored vs discovered edges on both the path-step and standalone viewers. Three markdown EPR nodes (constitution/confession/theology) are seeded with `relatedNodeIds` mirroring the cites mesh.

**Tech Stack:** Rust (diesel/SQLite, ts-rs, axum), Angular 19 (standalone components, vitest), genesis seeder (tsx), a2o (Cucumber/Cypress).

**Spec:** `genesis/docs/superpowers/specs/2026-06-08-native-content-graph-seam-design.md` (id `native-content-graph-seam-design`).

---

## Build & Test Conventions (read once)

- **Branch:** all work on `feat/native-content-graph-seam` (already created). Commit per task.
- **elohim-storage cargo:** from `/projects/elohim/elohim/elohim-storage`, set the pool slot and KEEP the default RUSTFLAGS (the custom getrandom backend — do NOT clear it for elohim-storage):
  ```bash
  export CARGO_TARGET_DIR="$(cargo-pool key)"   # per-worktree pool slot; avoids 30G legacy target balloon
  cargo test --lib <filter>                      # plain cargo (this container has no working nextest)
  ```
- **ts-rs export** (from `/projects/elohim/elohim/elohim-views`): `CARGO_TARGET_DIR="$(cargo-pool key)" cargo test export_bindings`.
- **Schema codegen** (repo root): `pnpm run schema:codegen:ts` and `pnpm run schema:validate`.
- **lamad Angular** (from `/projects/elohim/app/lamad`): confirm the runner in `package.json`; tests are vitest — `pnpm exec vitest run <pattern>` (verify the `--config` path from package.json `test` script before first run).
- **seeder** (from `/projects/elohim/genesis/seeder`): `pnpm run validate`, then targeted seed (Task C2).
- **a2o** (from `/projects/elohim/genesis/a2o` or `app/elohim-app` per its README): the Cucumber feature runner; confirm the exact invocation from the a2o package scripts before running.
- If a `cargo` build hits a `/projects-volume` fingerprint ENOENT, fall back to a `/tmp` target dir for that invocation (known container quirk).

---

## Phase A — Rust truth layer (the seam)

### Task A1: Wire schema contract for the graph response (schema-first)

**Files:**
- Create: `elohim/sdk/schemas/v1/views/content-graph.schema.json`
- Reference: `elohim/sdk/schemas/v1/views/CONVENTIONS.md`, an existing sibling schema (e.g. `content-view.schema.json`) for the exact envelope conventions.

- [ ] **Step 1: Read an existing view schema** to match conventions (camelCase, `$schema`, `additionalProperties`, required arrays). Run: `ls elohim/sdk/schemas/v1/views/ && sed -n '1,60p' elohim/sdk/schemas/v1/views/CONVENTIONS.md`

- [ ] **Step 2: Write `content-graph.schema.json`** — the contract the promoted view must satisfy:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "content-graph.schema.json",
  "title": "ContentGraph",
  "type": "object",
  "additionalProperties": false,
  "required": ["rootId", "related", "totalNodes"],
  "properties": {
    "rootId": { "type": "string" },
    "totalNodes": { "type": "integer", "minimum": 0 },
    "related": {
      "type": "array",
      "items": { "$ref": "#/definitions/contentGraphNode" }
    }
  },
  "definitions": {
    "contentGraphNode": {
      "type": "object",
      "additionalProperties": false,
      "required": ["contentId", "relationshipType", "confidence", "inferenceSource", "depth", "children"],
      "properties": {
        "contentId": { "type": "string" },
        "relationshipType": { "type": "string" },
        "confidence": { "type": "number" },
        "inferenceSource": { "type": "string", "enum": ["explicit", "path", "tag", "semantic", "system"] },
        "depth": { "type": "integer", "minimum": 1 },
        "children": { "type": "array", "items": { "$ref": "#/definitions/contentGraphNode" } }
      }
    }
  }
}
```

- [ ] **Step 3: Validate the schema file parses.** Run: `pnpm run schema:validate 2>&1 | tail -20`. Expected: no parse error for `content-graph.schema.json` (view-contract mismatch with Rust is expected until A6 — that is fine at this step).

- [ ] **Step 4: Commit.**
```bash
git add elohim/sdk/schemas/v1/views/content-graph.schema.json
git commit -m "feat(storage): schema-first contract for ContentGraph view (inferenceSource, depth)"
```

---

### Task A2: The `ContentGraphResolver` trait + value types

**Files:**
- Create: `elohim/elohim-storage/src/graph_engine.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (or `main.rs`/module root — wherever `mod` declarations live) to add `pub mod graph_engine;`

- [ ] **Step 1: Confirm the module root.** Run: `rg -n "pub mod services;|mod services;" elohim/elohim-storage/src/lib.rs`. Add the new module beside it.

- [ ] **Step 2: Write the trait + types** (read-only by construction — no write method):

```rust
// elohim/elohim-storage/src/graph_engine.rs
//! The native content-graph seam. The ONE place the content graph is realized.
//! Composes explicit (notarized, Category A) + computed (recompute-on-read,
//! Category C) edges. Read-only: this trait has no write method by design —
//! a computed edge can never be persisted through it.

use crate::db::context::AppContext;
use crate::error::StorageError;

/// One edge in a resolved neighborhood, discriminated by `inference_source`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEdge {
    pub target_id: String,
    pub relationship_type: String, // RELATES_TO for both classes in this slice
    pub confidence: f64,
    pub inference_source: String,  // "explicit" (A) | "tag" (C). Never persisted for C.
    pub depth: u32,                // 1 = direct; >1 = transitively-reached explicit edge
}

/// A resolved neighborhood rooted at one node.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedNeighborhood {
    pub root_id: String,
    pub edges: Vec<ResolvedEdge>,
}

/// Bounded knobs for one resolution.
#[derive(Debug, Clone)]
pub struct GraphQuery<'a> {
    pub root_id: &'a str,
    pub max_depth: u32,
    pub relationship_types: Option<&'a [String]>,
    pub include_computed: bool,
    pub max_computed: usize,
    pub min_shared_tags: usize,
}

impl<'a> GraphQuery<'a> {
    /// Defaults: depth 2 (cap enforced by caller), computed on, 25 cap, 1 shared tag.
    pub fn new(root_id: &'a str) -> Self {
        Self {
            root_id,
            max_depth: 2,
            relationship_types: None,
            include_computed: true,
            max_computed: 25,
            min_shared_tags: 1,
        }
    }
}

/// The seam. A future Cozo/datalog/embedding engine is just another impl.
pub trait ContentGraphResolver: Send + Sync {
    fn resolve_neighborhood(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<ResolvedNeighborhood, StorageError>;
}
```

- [ ] **Step 3: Add `pub mod graph_engine;`** to the module root identified in Step 1.

- [ ] **Step 4: Compile.** Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo build --lib 2>&1 | tail -20`. Expected: builds (unused-trait warnings OK). Fix any import path errors (`AppContext`, `StorageError`) by matching the paths used in `services/relationship_service.rs`.

- [ ] **Step 5: Commit.**
```bash
git add elohim/elohim-storage/src/graph_engine.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage): ContentGraphResolver trait + resolved-edge value types"
```

---

### Task A3: `NativeGraphResolver` Pass 1 — explicit depth-bounded BFS (de-stub depth>1)

**Files:**
- Modify: `elohim/elohim-storage/src/graph_engine.rs`
- Reference: `elohim/elohim-storage/src/db/relationships_diesel.rs` (`get_outgoing_relationships(conn, ctx, content_id, relationship_types) -> Vec<Relationship>`), `db/models.rs` `Relationship` (fields `target_id`, `relationship_type`, `confidence: f32`, `inference_source: String`).
- Test: inline `#[cfg(test)]` in `graph_engine.rs` (mirror the in-memory-SQLite harness used in `db/relationships_diesel.rs` tests — read that test module first for the pool/migration setup helper).

- [ ] **Step 1: Read the existing diesel test harness** so the resolver test reuses it. Run: `rg -n "#\[cfg\(test\)\]|fn .*test.*pool|run_migrations|establish" elohim/elohim-storage/src/db/relationships_diesel.rs | head`.

- [ ] **Step 2: Write the failing test** — explicit BFS reaches depth-2 (this is the behavior the current stub lacks). Add to `graph_engine.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Reuse the in-memory pool + migration helper from the db test harness.
    // A→B (depth 1), B→C (depth 1 from B) => from A at depth 2, edges to B(d1) and C(d2).

    #[test]
    fn explicit_bfs_reaches_depth_two() {
        let (pool, ctx) = crate::db::relationships_diesel::tests::test_pool_with_ctx(); // adapt name
        seed_rel(&pool, &ctx, "A", "B", "RELATES_TO", "explicit");
        seed_rel(&pool, &ctx, "B", "C", "RELATES_TO", "explicit");
        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery { include_computed: false, max_depth: 2, ..GraphQuery::new("A") };
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();
        let targets: std::collections::BTreeSet<_> =
            n.edges.iter().map(|e| (e.target_id.as_str(), e.depth)).collect();
        assert!(targets.contains(&("B", 1)), "B at depth 1");
        assert!(targets.contains(&("C", 2)), "C at depth 2 (de-stubbed)");
        assert!(n.edges.iter().all(|e| e.inference_source == "explicit"));
    }
}
```
(Adapt `test_pool_with_ctx` / `seed_rel` to the actual helper names found in Step 1; if the db test module is private, add a small `pub(crate)` test helper there.)

- [ ] **Step 3: Run it — verify it fails to compile/assert** (`NativeGraphResolver` undefined). Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine::tests::explicit_bfs 2>&1 | tail -15`. Expected: FAIL (unresolved `NativeGraphResolver`).

- [ ] **Step 4: Implement `NativeGraphResolver` + Pass 1** in `graph_engine.rs`:

```rust
use std::collections::{HashSet, VecDeque};
use crate::db::DbPool;
use crate::db::relationships_diesel;

pub struct NativeGraphResolver {
    pool: DbPool,
}

impl NativeGraphResolver {
    pub fn new(pool: DbPool) -> Self { Self { pool } }

    fn conn(&self) -> Result<
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>,
        StorageError,
    > {
        self.pool.get().map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))
    }

    /// Pass 1: explicit edges via depth-bounded BFS over stored relationships.
    fn explicit_edges(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<Vec<ResolvedEdge>, StorageError> {
        let mut conn = self.conn()?;
        let depth_cap = query.max_depth.min(3); // hard cap
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(query.root_id.to_string());
        let mut out: Vec<ResolvedEdge> = Vec::new();
        let mut frontier: VecDeque<(String, u32)> = VecDeque::new();
        frontier.push_back((query.root_id.to_string(), 0));

        while let Some((node, depth)) = frontier.pop_front() {
            if depth >= depth_cap { continue; }
            let rels = relationships_diesel::get_outgoing_relationships(
                &mut conn, ctx, &node, query.relationship_types,
            )?;
            for r in rels {
                if visited.insert(r.target_id.clone()) {
                    let edge_depth = depth + 1;
                    out.push(ResolvedEdge {
                        target_id: r.target_id.clone(),
                        relationship_type: r.relationship_type.clone(),
                        confidence: r.confidence as f64,
                        inference_source: if r.inference_source.is_empty() {
                            "explicit".to_string()
                        } else { r.inference_source.clone() },
                        depth: edge_depth,
                    });
                    frontier.push_back((r.target_id, edge_depth));
                }
            }
        }
        Ok(out)
    }
}

impl ContentGraphResolver for NativeGraphResolver {
    fn resolve_neighborhood(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<ResolvedNeighborhood, StorageError> {
        let edges = self.explicit_edges(ctx, query)?;
        Ok(ResolvedNeighborhood { root_id: query.root_id.to_string(), edges })
    }
}
```
(Match `DbPool`, `get_outgoing_relationships`, and the `Relationship` field types to the actual signatures from Step 1 — especially whether `confidence` is `f32` and whether `inference_source` is `String` vs `Option<String>`.)

- [ ] **Step 5: Run the test — verify PASS.** Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine::tests::explicit_bfs 2>&1 | tail -15`. Expected: PASS.

- [ ] **Step 6: Commit.**
```bash
git add elohim/elohim-storage/src/graph_engine.rs
git commit -m "feat(storage): NativeGraphResolver Pass 1 — explicit depth-bounded BFS (de-stubs depth>1)"
```

---

### Task A4: `NativeGraphResolver` Pass 2 — tag co-occurrence discovery (Category C)

**Files:**
- Modify: `elohim/elohim-storage/src/graph_engine.rs`
- Reference: `db/diesel_schema.rs` `content_tags (h_app_id, content_id, tag)`; `db/content_diesel.rs` `get_content_tags`. The `h_app_id` scoping comes from `AppContext` (confirm the accessor used elsewhere, e.g. `ctx.h_app_id()` or a field).

- [ ] **Step 1: Read how `h_app_id` is obtained from `AppContext`** and how raw SQL is run in this crate. Run: `rg -n "h_app_id|sql_query|QueryableByName" elohim/elohim-storage/src/db/content_diesel.rs elohim/elohim-storage/src/db/context.rs | head`.

- [ ] **Step 2: Write the failing test** — discovery surfaces a tag-neighbor with NO authored edge, tagged `inference_source="tag"`:

```rust
#[test]
fn tag_overlap_discovers_unlinked_neighbor() {
    let (pool, ctx) = crate::db::relationships_diesel::tests::test_pool_with_ctx();
    // X and Y share 2 tags but have NO relationship row.
    seed_content_with_tags(&pool, &ctx, "X", &["grace", "sin"]);
    seed_content_with_tags(&pool, &ctx, "Y", &["grace", "sin"]);
    let resolver = NativeGraphResolver::new(pool);
    let q = GraphQuery { include_computed: true, min_shared_tags: 2, ..GraphQuery::new("X") };
    let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();
    let disc: Vec<_> = n.edges.iter().filter(|e| e.inference_source == "tag").collect();
    assert!(disc.iter().any(|e| e.target_id == "Y"), "Y discovered via shared tags");
    assert!(disc.iter().all(|e| e.depth == 1));
}
```
(Add `seed_content_with_tags` helper using `content_diesel`/raw inserts into `content` + `content_tags`.)

- [ ] **Step 3: Run — verify FAIL** (no tag pass yet). Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine::tests::tag_overlap 2>&1 | tail -15`. Expected: FAIL.

- [ ] **Step 4: Implement Pass 2** and compose it in `resolve_neighborhood`:

```rust
#[derive(diesel::QueryableByName)]
struct TagOverlapRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    content_id: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    shared: i64,
}

impl NativeGraphResolver {
    /// Pass 2: tag co-occurrence. Recompute-on-read; NEVER persisted (Category C).
    fn computed_tag_edges(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
        exclude: &std::collections::HashSet<String>,
    ) -> Result<Vec<ResolvedEdge>, StorageError> {
        use diesel::prelude::*;
        let mut conn = self.conn()?;
        let app = ctx.h_app_id(); // adapt accessor
        let root_tag_count: i64 = /* COUNT(*) FROM content_tags WHERE h_app_id=? AND content_id=? */
            diesel::sql_query(
                "SELECT COUNT(*) AS shared, '' AS content_id FROM content_tags \
                 WHERE h_app_id = ?1 AND content_id = ?2")
            .bind::<diesel::sql_types::Text, _>(&app)
            .bind::<diesel::sql_types::Text, _>(query.root_id)
            .get_result::<TagOverlapRow>(&mut conn)
            .map(|r| r.shared).unwrap_or(0);
        let rows: Vec<TagOverlapRow> = diesel::sql_query(
            "SELECT ct2.content_id AS content_id, COUNT(*) AS shared \
             FROM content_tags ct1 \
             JOIN content_tags ct2 ON ct1.tag = ct2.tag \
               AND ct1.h_app_id = ct2.h_app_id AND ct2.content_id <> ct1.content_id \
             WHERE ct1.h_app_id = ?1 AND ct1.content_id = ?2 \
             GROUP BY ct2.content_id HAVING shared >= ?3 \
             ORDER BY shared DESC LIMIT ?4")
            .bind::<diesel::sql_types::Text, _>(&app)
            .bind::<diesel::sql_types::Text, _>(query.root_id)
            .bind::<diesel::sql_types::BigInt, _>(query.min_shared_tags as i64)
            .bind::<diesel::sql_types::BigInt, _>(query.max_computed as i64)
            .load(&mut conn)?;
        let denom = root_tag_count.max(1) as f64;
        Ok(rows.into_iter()
            .filter(|r| !exclude.contains(&r.content_id))
            .map(|r| ResolvedEdge {
                target_id: r.content_id,
                relationship_type: "RELATES_TO".to_string(),
                confidence: (r.shared as f64 / denom).clamp(0.0, 1.0),
                inference_source: "tag".to_string(),
                depth: 1,
            })
            .collect())
    }
}
```
Update `resolve_neighborhood`:
```rust
fn resolve_neighborhood(&self, ctx, query) -> Result<ResolvedNeighborhood, StorageError> {
    let explicit = self.explicit_edges(ctx, query)?;
    let mut edges = explicit.clone();
    if query.include_computed {
        let mut seen: std::collections::HashSet<String> =
            explicit.iter().map(|e| e.target_id.clone()).collect();
        seen.insert(query.root_id.to_string());
        let computed = self.computed_tag_edges(ctx, query, &seen)?;
        edges.extend(computed); // explicit precedence preserved by `seen`
    }
    Ok(ResolvedNeighborhood { root_id: query.root_id.to_string(), edges })
}
```
(Adapt the dialect: SQLite bind placeholders may be `?` not `?1`/`?2` depending on diesel version — verify against an existing `sql_query` in the crate.)

- [ ] **Step 5: Run — verify PASS;** also re-run A3 test (no regression). Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine::tests 2>&1 | tail -20`. Expected: both PASS.

- [ ] **Step 6: Add explicit-precedence + include_computed=false tests.**
```rust
#[test]
fn explicit_precedence_over_computed() {
    let (pool, ctx) = crate::db::relationships_diesel::tests::test_pool_with_ctx();
    seed_content_with_tags(&pool, &ctx, "X", &["grace"]);
    seed_content_with_tags(&pool, &ctx, "Y", &["grace"]);
    seed_rel(&pool, &ctx, "X", "Y", "RELATES_TO", "explicit"); // authored edge X→Y
    let resolver = NativeGraphResolver::new(pool);
    let q = GraphQuery { min_shared_tags: 1, ..GraphQuery::new("X") };
    let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();
    let y: Vec<_> = n.edges.iter().filter(|e| e.target_id == "Y").collect();
    assert_eq!(y.len(), 1, "Y appears once");
    assert_eq!(y[0].inference_source, "explicit", "explicit wins over tag");
}

#[test]
fn include_computed_false_yields_no_tag_edges() {
    let (pool, ctx) = crate::db::relationships_diesel::tests::test_pool_with_ctx();
    seed_content_with_tags(&pool, &ctx, "X", &["grace"]);
    seed_content_with_tags(&pool, &ctx, "Y", &["grace"]);
    let resolver = NativeGraphResolver::new(pool);
    let q = GraphQuery { include_computed: false, ..GraphQuery::new("X") };
    let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();
    assert!(n.edges.iter().all(|e| e.inference_source != "tag"));
}
```
Run them: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine::tests 2>&1 | tail -20`. Expected: all PASS.

- [ ] **Step 7: Commit.**
```bash
git add elohim/elohim-storage/src/graph_engine.rs
git commit -m "feat(storage): NativeGraphResolver Pass 2 — tag co-occurrence discovery (Category C, recompute-on-read)"
```

---

### Task A5: `GraphSpec` loader — honor `graph.json` as the declarative model

**Files:**
- Create: `elohim/elohim-storage/src/graph_engine_spec.rs` (or a `spec` submodule in `graph_engine.rs`)
- Reference: `elohim/sdk/domains/lamad/manifest/graph.json` (`edges[].type`, `indexes[]`, `rules[]`)

- [ ] **Step 1: Decide load strategy.** The manifest is a build-time asset. Embed it with `include_str!` (relative path from the crate) to avoid a runtime file dependency. Confirm the relative path: `realpath --relative-to=elohim/elohim-storage/src elohim/sdk/domains/lamad/manifest/graph.json`.

- [ ] **Step 2: Write the failing test** — the spec exposes the declared edge-type whitelist:
```rust
#[test]
fn graph_spec_exposes_edge_vocabulary() {
    let spec = GraphSpec::load();
    let kinds = spec.edge_types();
    for k in ["PREREQUISITE", "TEACHES", "CONTAINS", "REFERENCES", "SUPERSEDES"] {
        assert!(kinds.contains(&k.to_string()), "{k} declared in graph.json");
    }
}
```

- [ ] **Step 3: Run — FAIL.** Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine_spec 2>&1 | tail -15`. Expected: FAIL.

- [ ] **Step 4: Implement `GraphSpec`:**
```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawEdge { #[serde(rename = "type")] kind: String }
#[derive(Debug, Deserialize)]
struct RawSpec { edges: Vec<RawEdge> }

pub struct GraphSpec { edge_types: Vec<String> }

impl GraphSpec {
    pub fn load() -> Self {
        const RAW: &str = include_str!("../../sdk/domains/lamad/manifest/graph.json"); // adapt path
        let parsed: RawSpec = serde_json::from_str(RAW).expect("graph.json parses");
        Self { edge_types: parsed.edges.into_iter().map(|e| e.kind).collect() }
    }
    pub fn edge_types(&self) -> &[String] { &self.edge_types }
}
```

- [ ] **Step 5: Wire the whitelist into the resolver** — when `query.relationship_types` is `None`, default Pass-1 traversal to `RELATES_TO` plus any `GraphSpec::edge_types()` present (the seeded reality is `RELATES_TO`; the spec is the source of *which kinds exist*). Add a `NativeGraphResolver::new_with_spec(pool, spec)` or load the spec once in `new`. Keep behavior identical for the existing tests (they pass explicit `RELATES_TO` rows).

- [ ] **Step 6: Run — PASS** (spec test + all prior). Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo test --lib graph_engine 2>&1 | tail -20`. Expected: PASS.

- [ ] **Step 7: Commit.**
```bash
git add elohim/elohim-storage/src/graph_engine*.rs
git commit -m "feat(storage): GraphSpec loader — graph.json edge vocabulary drives native traversal"
```

---

### Task A6: Promote `ContentGraph` to a ts-rs view (+`inferenceSource`,`depth`) and convert from the resolver

**Files:**
- Create/modify: `elohim/elohim-views/src/lamad.rs` (or the content domain module — confirm where `ContentView` lives) — add `ContentGraphView` + `ContentGraphNodeView`
- Modify: `elohim/elohim-storage/src/views.rs` — add `From<ResolvedNeighborhood>` (+ node `From`) conversion (the converter touches DB-free resolver output → view; per CLAUDE.md `views.rs` is the From-shim home)
- Modify: `elohim/elohim-storage/src/services/relationship_service.rs` — delete the local `ContentGraph`/`ContentGraphNode` structs (lines ~366–381)
- Reference: `elohim-storage/CLAUDE.md` "Adding New Entities", the `#[ts(export, export_to=...)]` pattern.

- [ ] **Step 1: Find the views home + an example.** Run: `rg -n "ts\(export" elohim/elohim-views/src/lamad.rs | head; rg -n "ContentView" elohim/elohim-views/src/*.rs | head`.

- [ ] **Step 2: Add the view types** in `elohim-views`:
```rust
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentGraphNodeView {
    pub content_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub inference_source: String,
    pub depth: u32,
    pub children: Vec<ContentGraphNodeView>,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentGraphView {
    pub root_id: String,
    pub related: Vec<ContentGraphNodeView>,
    pub total_nodes: usize,
}
```
(Match the `ts_rs::TS` import idiom already used in the file.)

- [ ] **Step 3: Add the conversion** in `views.rs`:
```rust
use crate::graph_engine::{ResolvedEdge, ResolvedNeighborhood};
use elohim_views::lamad::{ContentGraphNodeView, ContentGraphView}; // adapt path

impl From<&ResolvedEdge> for ContentGraphNodeView {
    fn from(e: &ResolvedEdge) -> Self {
        Self {
            content_id: e.target_id.clone(),
            relationship_type: e.relationship_type.clone(),
            confidence: e.confidence,
            inference_source: e.inference_source.clone(),
            depth: e.depth,
            children: vec![], // flat for this slice
        }
    }
}

impl From<ResolvedNeighborhood> for ContentGraphView {
    fn from(n: ResolvedNeighborhood) -> Self {
        let related: Vec<ContentGraphNodeView> = n.edges.iter().map(Into::into).collect();
        let total_nodes = related.len();
        Self { root_id: n.root_id, related, total_nodes }
    }
}
```

- [ ] **Step 4: Delete the old service structs** (`ContentGraph`, `ContentGraphNode`) from `relationship_service.rs:366–381`. The service methods will be rewired in A8 — temporarily, make `get_graph`/`get_graph_with_depth` return `ContentGraphView` by mapping (compile-bridge until A8 fully delegates).

- [ ] **Step 5: Regenerate TS bindings.** Run: `cd elohim/elohim-views && CARGO_TARGET_DIR="$(cargo-pool key)" cargo test export_bindings 2>&1 | tail -10`. Expected: PASS; `ContentGraphView.ts`/`ContentGraphNodeView.ts` written under `sdk/storage-client-ts/src/generated/`.

- [ ] **Step 6: Run the schema contract.** Run: `cd elohim/elohim-storage && CARGO_TARGET_DIR="$(cargo-pool key)" cargo test --test schema_contract 2>&1 | tail -20`. If `content-graph` isn't checked yet, that's Task A9 — at minimum confirm no compile break.

- [ ] **Step 7: Commit.**
```bash
git add elohim/elohim-views/src/ elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/services/relationship_service.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): promote ContentGraph to ts-rs view (inferenceSource, depth); fixes latent camelCase drop"
```

---

### Task A7: Canonicalize the `inference_source` vocabulary to TS

**Files:**
- Reference: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:849` (`INFERENCE_SOURCES`), `genesis/seeder/src/generated/schema-enums.ts`, the lamad TS `RelationshipInferenceSource` (`app/lamad/src/app/models/content-node.model.ts:699–705`)
- Modify (TS, Phase B touches the consumer): document the canonical set so B1 maps against it.

- [ ] **Step 1: Confirm the canonical set** is `explicit | path | tag | semantic` (DHT) + `system` (storage). Run: `rg -n "INFERENCE_SOURCES|valid_sources" elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs elohim/elohim-storage/src/services/relationship_service.rs`.

- [ ] **Step 2: Decision (record in commit):** the slice EMITS only `explicit` and `tag`. The schema enum (A1) already lists the canonical five. The lamad TS enum is consumer-side drift handled in B1 via an `isDiscovered()` predicate (no wire reshape). No Rust change needed here beyond confirming the resolver only emits canonical values (it does: `"explicit"`, `"tag"`). This task is a verification gate, not new code — keep it as a documented checkpoint so the Angular side maps correctly.

- [ ] **Step 3: Commit** (doc-only if any notes added; otherwise skip — checkpoint folds into B1).

---

### Task A8: Wire `RelationshipService` to the resolver + route delta

**Files:**
- Modify: `elohim/elohim-storage/src/services/relationship_service.rs` — hold `Arc<dyn ContentGraphResolver>`; `get_graph`/`get_graph_with_depth` build a `GraphQuery` and delegate, returning `ContentGraphView`
- Modify: wherever `RelationshipService::new` is constructed (find the call site) to inject `Arc::new(NativeGraphResolver::new(pool.clone()))`
- Modify: `elohim/elohim-storage/src/http.rs` `handle_db_content_graph` (~4206) — parse new query params `depth|computed|minSharedTags|maxComputed`

- [ ] **Step 1: Find the construction site + the route handler signature.** Run: `rg -n "RelationshipService::new|handle_db_content_graph|relationships/graph" elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/**/*.rs | head`.

- [ ] **Step 2: Rewire the service.** Add field `resolver: Arc<dyn ContentGraphResolver>`; in `new(...)` accept or build it. Replace `get_graph` body:
```rust
pub fn get_graph(&self, content_id: &str, relationship_types: Option<&[String]>)
    -> Result<ContentGraphView, StorageError> {
    let q = GraphQuery { relationship_types, max_depth: 1, include_computed: false, ..GraphQuery::new(content_id) };
    Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
}
pub fn get_graph_with_depth(&self, content_id: &str, max_depth: u32,
    relationship_types: Option<&[String]>) -> Result<ContentGraphView, StorageError> {
    let q = GraphQuery { relationship_types, max_depth, ..GraphQuery::new(content_id) };
    Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
}
```
Add a richer entrypoint used by the route:
```rust
pub fn get_graph_query(&self, content_id: &str, depth: u32, include_computed: bool,
    min_shared_tags: usize, max_computed: usize, relationship_types: Option<&[String]>)
    -> Result<ContentGraphView, StorageError> {
    let q = GraphQuery { root_id: content_id, max_depth: depth, relationship_types,
        include_computed, max_computed, min_shared_tags };
    Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
}
```

- [ ] **Step 3: Extend the route's query struct** in `http.rs` (camelCase per the crate convention):
```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentGraphParams {
    pub types: Option<String>,        // existing comma list
    pub depth: Option<u32>,
    pub computed: Option<bool>,
    pub min_shared_tags: Option<usize>,
    pub max_computed: Option<usize>,
}
```
In `handle_db_content_graph`, parse and call `get_graph_query(id, depth.unwrap_or(2).min(3), computed.unwrap_or(true), min_shared_tags.unwrap_or(1), max_computed.unwrap_or(25), types_vec)`. Keep the handler thin.

- [ ] **Step 4: Build.** Run: `export CARGO_TARGET_DIR="$(cargo-pool key)"; cargo build --lib 2>&1 | tail -20`. Fix construction-site arity. Expected: builds.

- [ ] **Step 5: Add a route-level test** (or extend an existing http test) asserting `?computed=true&minSharedTags=1` returns a body with a `tag`-source node for a fixture with two tag-sharing docs and no authored edge. If http handler tests are heavy here, assert at the service layer via `get_graph_query`. Run the targeted test; Expected: PASS.

- [ ] **Step 6: Commit.**
```bash
git add elohim/elohim-storage/src/services/relationship_service.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): RelationshipService delegates to ContentGraphResolver; /db/relationships/graph gains depth+computed params"
```

---

### Task A9: Schema-contract + codegen freshness

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract.rs` — add a `content-graph` case
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` — add `content-graph` to `INTERFACE_FILES`

- [ ] **Step 1: Add the contract case.** Read an existing case in `schema_contract.rs`, add one validating `ContentGraphView`'s serialized shape against `content-graph.schema.json`.

- [ ] **Step 2: Run the contract test.** Run: `cd elohim/elohim-storage && CARGO_TARGET_DIR="$(cargo-pool key)" cargo test --test schema_contract 2>&1 | tail -20`. Expected: PASS (fix any field-name/enum mismatch between A1 schema and A6 view).

- [ ] **Step 3: Add to `INTERFACE_FILES` and regen.** Run: `pnpm run schema:codegen:ts 2>&1 | tail -10`. Expected: clean; `git status` shows only intended generated changes (watch the codegen Prettier oscillation note — cosmetic-only churn on unrelated files should be reverted).

- [ ] **Step 4: Commit.**
```bash
git add elohim/elohim-storage/tests/schema_contract.rs elohim/sdk/schemas/scripts/codegen-ts.mjs elohim/sdk/storage-client-ts/src/generated/
git commit -m "test(storage): content-graph schema contract + codegen freshness"
```

---

## Phase B — Angular (shared sidebar + discovery rendering)

### Task B1: `inferenceSource` plumbing + `isDiscovered` + `discovered` bucket

**Files:**
- Modify: `app/lamad/src/app/models/content-node.model.ts` — add `inferenceSource?` to `ContentRelationship` (~516–522); add `discovered: ContentNode[]` to `RelatedConceptsResult`
- Modify: `app/lamad/src/app/services/data-loader.service.ts` — carry `inferenceSource` in `transformToContentRelationship` (~1436–1451)
- Modify: `app/lamad/src/app/services/related-concepts.service.ts` — `isDiscovered()` predicate + route discovered edges into `discovered` in `categorizeRelationships` (~357)
- Test: the related-concepts.service spec

- [ ] **Step 1: Write the failing test** — a `tag`-source relationship lands in `discovered`, an `explicit` one does not:
```ts
it('routes tag-source edges into discovered', () => {
  const rels = [
    { sourceNodeId: 'confession', targetNodeId: 'theology', relationshipType: 'RELATES_TO', inferenceSource: 'explicit' },
    { sourceNodeId: 'confession', targetNodeId: 'some-tagmate', relationshipType: 'RELATES_TO', inferenceSource: 'tag' },
  ] as ContentRelationship[];
  const result = service.categorizeRelationships('confession', rels, nodeLookup);
  expect(result.discovered.map(n => n.id)).toContain('some-tagmate');
  expect(result.discovered.map(n => n.id)).not.toContain('theology');
});
```

- [ ] **Step 2: Run — FAIL.** Run (from `app/lamad`): `pnpm exec vitest run related-concepts.service 2>&1 | tail -20`. Expected: FAIL (`discovered` undefined).

- [ ] **Step 3: Implement.** Add `inferenceSource?: string` to the model; pass it through `transformToContentRelationship`; add:
```ts
private isDiscovered(src?: string): boolean {
  return src != null && src !== 'explicit' && src !== 'author';
}
```
In `categorizeRelationships`, before type-routing: `if (this.isDiscovered(rel.inferenceSource)) { discovered.push(node); continue; }`. Initialize `discovered: ContentNode[] = []` and include it in the returned `RelatedConceptsResult`.

- [ ] **Step 4: Run — PASS.** Run: `pnpm exec vitest run related-concepts.service 2>&1 | tail -20`. Expected: PASS.

- [ ] **Step 5: Commit.**
```bash
git add app/lamad/src/app/models/content-node.model.ts app/lamad/src/app/services/data-loader.service.ts app/lamad/src/app/services/related-concepts.service.ts app/lamad/src/app/services/related-concepts.service.spec.ts
git commit -m "feat(lamad): carry inferenceSource to the panel; isDiscovered routes computed edges to a discovered bucket"
```

---

### Task B2: `RelatedConceptsPanelComponent` — the "Discovered" section

**Files:**
- Modify: `app/lamad/src/app/components/related-concepts-panel/related-concepts-panel.component.ts` (+ template) — render `discovered` after the authored sections, with `data-testid="discovered-concept-card"` and an `inference-source` attribute.
- Test: the panel spec

- [ ] **Step 1: Write the failing test** — discovered nodes render with the discovered testid. (Provide a `RelatedConceptsResult` with one `discovered` node via the service mock; assert `By.css('[data-testid="discovered-concept-card"]')` count = 1.)

- [ ] **Step 2: Run — FAIL.** Run: `pnpm exec vitest run related-concepts-panel 2>&1 | tail -20`.

- [ ] **Step 3: Implement** the "Discovered — you might also explore" section in the template (muted/dashed class), iterating `result.discovered`, each `ConceptCardComponent`/card carrying `data-testid="discovered-concept-card"` and `[attr.inference-source]="'tag'"`.

- [ ] **Step 4: Run — PASS.** Run: `pnpm exec vitest run related-concepts-panel 2>&1 | tail -20`.

- [ ] **Step 5: Commit.**
```bash
git add app/lamad/src/app/components/related-concepts-panel/
git commit -m "feat(lamad): related-concepts panel renders a Discovered section for computed edges"
```

---

### Task B3: `ExplorationSidebarComponent` (the shared wrapper)

**Files:**
- Create: `app/lamad/src/app/components/exploration-sidebar/exploration-sidebar.component.ts` (+ `.spec.ts`)
- Reference: `lesson-view.component.ts:209–264` (the aside to extract), `mini-graph`, `related-concepts-panel` IO contracts.

- [ ] **Step 1: Write the failing test** — renders mini-graph + related-concepts-panel + the explore button (`data-testid="exploration-explore-graph"`) given a `contentId`; emits `exploreInGraph` on click.

- [ ] **Step 2: Run — FAIL.** Run: `pnpm exec vitest run exploration-sidebar 2>&1 | tail -20`.

- [ ] **Step 3: Implement** the standalone OnPush wrapper with the IO contract from the spec (§5.1): `@Input({required:true}) contentId`, `collapsible`, `[(open)]`, `compact`, `relatedLimit`, `graphDepth`, `graphHeight`; `@Output exploreContent`, `exploreInGraph`, `openChange`. Template composes `<app-mini-graph [focusNodeId]="contentId" …>` + `<app-related-concepts-panel [contentId]="contentId" …>` + the explore button. Move the toggle/backdrop/responsive CSS here.

- [ ] **Step 4: Run — PASS.** Run: `pnpm exec vitest run exploration-sidebar 2>&1 | tail -20`.

- [ ] **Step 5: Commit.**
```bash
git add app/lamad/src/app/components/exploration-sidebar/
git commit -m "feat(lamad): shared ExplorationSidebarComponent (mini-graph + related-concepts + explore)"
```

---

### Task B4: `LessonViewComponent` uses the shared sidebar

**Files:**
- Modify: `app/lamad/src/app/components/lesson-view/lesson-view.component.ts` — replace the inline `<aside>` (209–264) + toggle (197–207) + backdrop (266–269) + panel CSS with `<app-exploration-sidebar>`, preserving its outputs to PathNavigator.

- [ ] **Step 1: Run lesson-view's existing spec first** to capture green baseline. Run: `pnpm exec vitest run lesson-view 2>&1 | tail -20`.

- [ ] **Step 2: Replace** the aside block with `<app-exploration-sidebar [contentId]="content.id" [collapsible]="true" [(open)]="explorationPanelOpen" (exploreContent)="exploreContent.emit($event)" (exploreInGraph)="exploreInGraph.emit()">`; import the component; delete the now-dead panel CSS and `onRelatedConceptClick`/`onGraphNodeClick` plumbing made redundant (keep the parent `@Output`s).

- [ ] **Step 3: Update/keep the spec** so it asserts `<app-exploration-sidebar>` renders and the `lesson-explore-graph` behavior now flows through `exploration-explore-graph`. Run: `pnpm exec vitest run lesson-view 2>&1 | tail -20`. Expected: PASS.

- [ ] **Step 4: Commit.**
```bash
git add app/lamad/src/app/components/lesson-view/
git commit -m "refactor(lamad): lesson-view renders the shared ExplorationSidebar"
```

---

### Task B5: `ContentViewerComponent` gains the sidebar; retire redundant surfaces; root mini-graph at content id

**Files:**
- Modify: `app/lamad/src/app/components/content-viewer/content-viewer.component.{ts,html}` — add `<app-exploration-sidebar [contentId]="node.id" [collapsible]="false">` to the content tab; remove the inline "Related Content" grid (html:400–419) + `loadRelatedNodes`/`relatedNodes` (ts:990–1011); remove the duplicate mini-graph + explore from the Network tab (html:802–829).
- Modify: `app/lamad/src/app/services/data-loader.service.ts:1504` — root `getGraph()`/the mini-graph neighborhood at the actual content id, not the hardcoded `'manifesto'`.

- [ ] **Step 1: Baseline** content-viewer spec. Run: `pnpm exec vitest run content-viewer 2>&1 | tail -20`.

- [ ] **Step 2: Root the mini-graph at the content id.** In `data-loader.service.ts`, replace the literal `'manifesto'` root with the passed-in `contentId`. Add a unit test asserting `getGraph('confession')` requests `/db/relationships/graph/confession` (mock ContentBackendService). Run: `pnpm exec vitest run data-loader 2>&1 | tail -20`. Expected: PASS.

- [ ] **Step 3: Add the sidebar to the content tab; retire the inline grid + dup mini-graph.** Wire `(exploreContent)` to the existing graph-node handler and `(exploreInGraph)="exploreInGraph()"`. Delete `loadRelatedNodes`, the `relatedNodes` field, the inline grid markup, and the Network-tab mini-graph/explore block. Keep `<app-epr-relationships-panel>` untouched.

- [ ] **Step 4: Update the spec** — assert `<app-exploration-sidebar>` present, inline `related-content` grid absent, `epr-relationships-panel` still present. Run: `pnpm exec vitest run content-viewer 2>&1 | tail -20`. Expected: PASS.

- [ ] **Step 5: Lint the lamad bundle.** Run (from `app/lamad`): `pnpm run lint 2>&1 | tail -20` (or the bundle's eslint script). Fix issues.

- [ ] **Step 6: Commit.**
```bash
git add app/lamad/src/app/components/content-viewer/ app/lamad/src/app/services/data-loader.service.ts app/lamad/src/app/services/data-loader.service.spec.ts
git commit -m "feat(lamad): standalone content-viewer renders the shared ExplorationSidebar; retire inline grid + dup mini-graph; root neighborhood at content id"
```

---

## Phase C — Content (the first witness)

### Task C1: Author the three doctrinal seed JSONs + the relatedNodeIds mesh

**Files:**
- Create: `genesis/data/lamad/content/constitution.json`, `confession.json`, `theology.json`
- Modify: `genesis/data/lamad/content/manifesto.json` — set `relatedNodeIds` to the other three
- Reference: `genesis/data/lamad/content/manifesto.json` (exact shape), source markdown `genesis/docs/content/elohim-protocol/{constitution,confession,theology}.md`

- [ ] **Step 1: For each new doc, build the JSON** by mirroring `manifesto.json` and inlining the source markdown body into `content` (read the source `.md`, JSON-escape it). Use these field values:
  - `confession.json`: `id:"confession"`, `did:"did:web:elohim.host:content:confession"`, `contentType:"reference"`, `reach:"commons"`, `activityPubType:"Article"`, `title:"The Confession: The Theology of the Elohim Protocol, Stated Plainly"`, `contentFormat:"markdown"`, `sourcePath:"confession.md"`, `tags:["theology","confession","doctrine","elohim-protocol"]`, `relatedNodeIds:["theology","manifesto","constitution"]`, `metadata:{category:"theology"}`, `stewardedBy:[{humanId:"human-matthew-manager",affinity:1,role:"author"}]`, `contributors:[]`.
  - `theology.json`: `id:"theology"`, `title:"The Theology: A Disputation on the Elohim Protocol"`, `tags:["theology","disputation","doctrine","elohim-protocol"]`, `relatedNodeIds:["manifesto","constitution","confession"]`, `metadata:{category:"theology"}`, rest analogous.
  - `constitution.json`: `id:"constitution"`, `title:` (use the constitution.md H1), `tags:["constitution","governance","doctrine","elohim-protocol"]`, `relatedNodeIds:["manifesto","confession","theology"]`, `metadata:{category:"governance"}`, rest analogous.
  - **Tag discipline (load-bearing for discovery):** the shared `theology`/`doctrine`/`elohim-protocol` tags across confession+theology(+manifesto if retagged) are what produce the computed tag-neighbor the a2o asserts. Ensure ≥2 tags overlap between confession and theology.
  - **Author the FULL node shape, not a stub** (genesis→protocol decomposition lens): include `did:web:elohim.host:content:<id>`, `openGraphMetadata`, `linkedData` (schema.org JSON-LD with `@id`/`identifier`), `stewardedBy` (`[{humanId:"human-matthew-manager",affinity:1,role:"author"}]`), and `contributors` — mirroring `manifesto.json` exactly. These nodes are destined to become durable P2P-datalayer nodes; author them as complete protocol nodes now so genesis data is already protocol-shaped (no later enrichment pass).

- [ ] **Step 2: Set manifesto's mesh.** Edit `manifesto.json:15` `relatedNodeIds: []` → `["constitution","confession","theology"]`. Optionally add `"theology"`,`"doctrine"` to its `tags` so manifesto also participates in tag-discovery (keeps the witness symmetric).

- [ ] **Step 3: Do NOT add any of these ids to `genesis/data/lamad/paths/*.json`.** Verify: `rg -n "confession|theology|constitution" genesis/data/lamad/paths/ || echo "clean — not path-attached"`. Expected: clean (manifesto may appear; the three new ones must not).

- [ ] **Step 4: Validate.** Run (from `genesis/seeder`): `pnpm run validate 2>&1 | tail -30`. Expected: the three new files pass reach + contentFormat enum validation.

- [ ] **Step 5: Commit.**
```bash
git add genesis/data/lamad/content/constitution.json genesis/data/lamad/content/confession.json genesis/data/lamad/content/theology.json genesis/data/lamad/content/manifesto.json
git commit -m "feat(content): seed constitution/confession/theology as commons markdown EPR nodes + cites-mesh relatedNodeIds"
```

---

### Task C2: Seed and verify the witness

**Files:** none (operational). Requires a running local stack or doorway-alpha credentials.

- [ ] **Step 1: Seed the three nodes** (targeted, content-only). Run (from `genesis/seeder`): `DOORWAY_URL=<url> DOORWAY_API_KEY=<key> npx tsx src/seed.ts --content-only --ids=constitution,confession,theology 2>&1 | tail -30`. Expected: 3 content nodes upserted; `relatedNodeIds` → `RELATES_TO` `inference_source='explicit'` rows created in the same flow.

- [ ] **Step 2: Verify content serves.** Run: `curl -s <doorway>/db/content/confession | jq '{id,contentFormat,reach}'`. Expected: `confession`, `markdown`, `public`/`commons` (ungated).

- [ ] **Step 3: Verify explicit edges.** Run: `curl -s "<doorway>/db/relationships/graph/confession?computed=false" | jq '.related[] | {contentId,inferenceSource}'`. Expected: theology/manifesto/constitution with `inferenceSource: "explicit"`.

- [ ] **Step 4: Verify computed discovery.** Run: `curl -s "<doorway>/db/relationships/graph/confession?computed=true&minSharedTags=2" | jq '.related[] | select(.inferenceSource=="tag") | .contentId'`. Expected: ≥1 tag-neighbor (e.g. `theology` would already be explicit; expect a non-authored tag-mate — confirm at least one node appears via tags that is NOT in relatedNodeIds, or relax `minSharedTags` to 1).

- [ ] **Step 5: No commit** (operational verification). Record results in the task notes / a2o background.

---

## Phase D — Story (a2o regression + testid-sync)

### Task D1: Page-model testids + the a2o feature

**Files:**
- Modify: `genesis/a2o/src/framework/pages/epr-content.page.ts` — selectors for `exploration-sidebar`, `exploration-explore-graph`, `related-concept-card`, `discovered-concept-card` (`inference-source`)
- Create: `genesis/a2o/features/content/exploration-sidebar.feature` (or extend `lamad/deep-link-delivery.feature`)
- Create/modify: the step definitions backing the new steps (find the existing EPR step file).

- [ ] **Step 1: Confirm testids landed in the components** (B2/B3/B5 added them). Run: `rg -n "exploration-sidebar|discovered-concept-card|exploration-explore-graph" app/lamad/src`. Expected: present (testid-sync — selectors and components land together).

- [ ] **Step 2: Add page-model selectors** in `epr-content.page.ts` mirroring those testids.

- [ ] **Step 3: Write the feature** (spec §7) — three scenarios: standalone viewer shows authored + discovered; same sidebar on the path-step flow; markdown renders independent of the epr-composite keystone.

- [ ] **Step 4: Implement/extend step definitions** for the new Then-steps (sidebar visible, lists authored ids, ≥1 discovered card, discovered card `inference-source != explicit`).

- [ ] **Step 5: Run the a2o feature** against the seeded stack. Run (from the a2o package per its README): the Cucumber runner filtered to `exploration-sidebar.feature`. Expected: PASS. (If the stack lacks the computed edge, revisit C1 tag overlap.)

- [ ] **Step 6: Commit.**
```bash
git add genesis/a2o/src/framework/pages/epr-content.page.ts genesis/a2o/features/content/exploration-sidebar.feature genesis/a2o/<step-defs>
git commit -m "test(a2o): exploration sidebar surfaces authored + discovered neighbors for the doctrinal corpus"
```

---

## Final verification (whole-slice)

- [ ] **Storage tests green:** from `elohim/elohim-storage` — `CARGO_TARGET_DIR="$(cargo-pool key)" cargo test --lib graph_engine 2>&1 | tail`; `cargo test --test schema_contract 2>&1 | tail`.
- [ ] **Clippy + fmt** on touched Rust: `CARGO_TARGET_DIR="$(cargo-pool key)" cargo clippy --lib 2>&1 | tail` (no new warnings); `cargo fmt --check`.
- [ ] **lamad unit tests + lint** green: `pnpm exec vitest run 2>&1 | tail`; `pnpm run lint`.
- [ ] **Codegen freshness:** `pnpm run schema:codegen:ts` produces no diff (idempotent, modulo the known cosmetic Prettier oscillation — revert cosmetic-only churn).
- [ ] **a2o feature** green against the seeded stack.
- [ ] **Backlog follow-ups filed** (do not silently drop): (1) relationship-*kind* 3-vocabulary reconciliation; (2) affinity-% badge lost from the retired inline grid; (3) semantic/embedding resolver impl + `mastery_frontier` rule (future `ContentGraphResolver` impls).

---

## Self-Review (run before execution)

- **Spec coverage:** Trait+seam (A2) ✓; explicit BFS / depth>1 (A3) ✓; tag discovery (A4) ✓; graph.json honored (A5) ✓; ts-rs view + camelCase fix (A6) ✓; inference_source canonicalization (A7+B1) ✓; route delta (A8) ✓; schema contract (A1/A9) ✓; shared sidebar both viewers (B3/B4/B5) ✓; discovered rendering (B1/B2) ✓; empty-trap (B5 keeps ContentBackendService path; mini-graph rooting) ✓; seed witness (C1/C2) ✓; a2o + testid-sync (D1) ✓; P2P-gate clean (no new entity/route/notarized field — A-tasks add only a read param + a view) ✓.
- **Placeholder scan:** SQL bind-placeholder dialect (`?` vs `?N`), `AppContext` accessor (`h_app_id()`), and the diesel test-harness helper names are flagged as "adapt to actual signature" — these are verification steps, not unfilled placeholders; each task's Step 1 reads the real signature first.
- **Type consistency:** `ResolvedEdge`/`ResolvedNeighborhood`/`GraphQuery` used identically across A2–A8; `ContentGraphView`/`ContentGraphNodeView` field names (`inferenceSource`,`depth`) match the A1 schema enum and the B1 consumer.
