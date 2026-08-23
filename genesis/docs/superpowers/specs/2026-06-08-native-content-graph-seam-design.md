---
id: native-content-graph-seam-design
cites:
  - "confession | Settled-theology doctrinal doc, seeded as the primary markdown EPR witness whose cites-mesh edges plus a computed tag-neighbor prove the resolver end-to-end | sha256:bec001fd41230c67 | path: genesis/docs/content/elohim-protocol/confession.md"
  - "theology | Disputation doctrinal doc, seeded as a markdown EPR witness for the shared exploration sidebar and the explicit-vs-discovered rendering | sha256:4daef6885e3cc420 | path: genesis/docs/content/elohim-protocol/theology.md"
  - "elohim-protocol-manifesto | Vision doctrinal doc, the only one already seeded as an EPR node; the cites mesh extends its relatedNodeIds to the other three as explicit RELATES_TO edges | sha256:cd62d3cc869bada5 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "constitution | Law doctrinal doc, newly seeded as a markdown EPR node so the four-doc graph resolves; an explicit-edge target in the cites mesh | sha256:1eb96af782012fc6 | path: genesis/docs/content/elohim-protocol/constitution.md"
  - "epr-route-claims-link-conformance-design | EPR routing, redirect governance and link-integrity conformance — the substrate that serves /epr/{slug} for the seeded docs and the route-claims context the standalone viewer depends on | sha256:30b7cd1baf222922 | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md"
---

# Native Content-Graph Seam — Bringing the Genesis Graph Model Alive

> **Status:** design (approved 2026-06-08)
> **Shape:** one vertical slice — Rust truth-layer seam + wire schema contract + cross-bundle Angular + seeded witness content + a2o regression
> **Theme:** genesis is model-rich; the native code is realization-poor. This slice stands up the *mechanism* that brings a piece of the rich genesis graph model genuinely alive in the DHT/peer/EPR-native, offline-correct world — and proves it with real computed discovery, using the doctrinal corpus as its first witness.

## 1. Motivation — the gap between model and realization

The genesis layer carries a sophisticated content-graph model that the native code does not yet realize:

- **`elohim/sdk/domains/lamad/manifest/graph.json`** declares a real graph model — 6 directional EPR edge types (PREREQUISITE/TEACHES/CONTAINS/REFERENCES/MASTERY_OF/SUPERSEDES), indexes, and two CozoScript datalog rules (`prerequisite_chain`, `mastery_frontier`). **Nothing executes it.** It is a spec written in the dialect of an engine (Cozo) that was never wired.
- The notarized DHT `Relationship` entry (`content_store_integrity`) carries an **`inference_source`** field (`explicit | path | tag | semantic`) anticipating *computed* discovery — but the only sources ever materialized are `explicit` (authored `relatedNodeIds`) and `structural` (path → CONTAINS). The `tag`/`semantic` discovery the model anticipates is **never computed**.
- `RelationshipService::get_graph` is hardcoded to **depth-1** (`get_graph_with_depth` is a literal stub) — the transitive model the datalog rules describe does not run.
- The `inference_source` vocabulary **drifts across three homes**: DHT `INFERENCE_SOURCES` (`explicit|path|tag|semantic`), storage `valid_sources` (`+system`), and a *hand-written* lamad TS enum (`author|structural|semantic|usage|citation|system`).
- The `ContentGraph` HTTP read shape is **un-contracted** — plain serde, snake_case, no ts-rs, no JSON schema — with a latent camelCase bug (`root_id` vs the Angular consumer's `rootId`) that silently drops fields today.

### Why native, not Cozo/Apollo/Kuzu (recorded rationale)

No design record ever named Cozo or Apollo — neither string appears in the repo. The lineage that *is* in code: **Kuzu** (embedded property-graph DB) was explored and **deprecated** (`"replaces Kuzu"`, `content_store_integrity/src/lib.rs:856,3888`); relationships migrated into Holochain DHT links + a SQLite projection. **Cozo** survives only as the dialect of the aspirational `graph.json` rules. **Apollo** is the wrong layer entirely — a centralized GraphQL *server* contradicts the hub-optional / laptop-is-a-full-participant / "k8s is not the architecture" floor. The operator's deliberate choice was to go **lower than any external engine: own graph coherence in the Rust storage services for mission-critical performance.** This slice realizes that as a clean interface so a future Cozo/datalog/embedding engine is *one more impl behind the trait*, never a rewrite.

## 2. Goals / Non-goals

**Goals**
1. Introduce `trait ContentGraphResolver` in `elohim-storage` — the single place the content graph is *realized*. Read-only by construction.
2. A `NativeGraphResolver` default impl that composes, in one response, **explicit** (notarized, authored) edges and **computed** (derived-on-read) discovery edges, discriminated by `inference_source`.
3. The first genuine computed-discovery signal: **tag co-occurrence** (`inference_source = "tag"`), plus a real depth-bounded traversal (kills the depth-1 stub).
4. Honor `graph.json` as the declarative model the native impl reads (edge vocabulary + index/rule intent).
5. **Canonicalize the `inference_source` vocabulary** on the DHT/storage home and generate it to TS — retire the hand-written lamad drift.
6. Promote `ContentGraph`/`ContentGraphNode` to first-class **ts-rs views under schema contract** (fixes the latent camelCase bug as a side effect).
7. A shared `ExplorationSidebarComponent` rendering the graph **beside content on both viewers** (path-step *and* standalone `/epr/{id}`), with computed edges visibly read as *discovery*.
8. Seed the doctrinal corpus (manifesto exists; add constitution/confession/theology) as `commons` markdown EPR nodes, not in the default path, with `relatedNodeIds` mirroring the cites mesh — the first witness.
9. Story-first a2o regression locking the experience.

**Non-goals (deferred *behind the same seam* — incremental realization, not a rewrite)**
- Semantic/embedding inference (`inference_source = "semantic"`) — a future resolver impl.
- The `mastery_frontier` rule (needs MASTERY_OF edges + contributor identity).
- The Cozo/datalog `ContentGraphResolver` impl (the whole point of the trait is that this is additive).
- Full relationship-*kind* reconciliation (manifest 11 / DHT 6 / storage 16) — sidestepped via `RELATES_TO` for this slice; tracked like the existing Reach-enum drift.
- Restoring the affinity-% badge lost when the inline related grid is retired (backlog).

## 3. P2P Design Gate (output)

### Entity: Doctrinal Content Node (`confession`, `theology`, `constitution`)
- **Classification:** Notarized (A) — reuses the existing content entry type (`manifesto` is already one).
- **Address:** Slug (`id: confession`) — established `/epr/{slug}` pattern; singletons; integrity via blob/provenance hash, not the navigation id.
- **Source of truth:** DHT content entry → SQLite projection. Coordinator/route: existing `/db/content/bulk` (seed) + doorway `GET /epr/{id}` (serve). **No new route.**

### Entity: Content↔Content Relationship (explicit, the cites mesh)
- **Classification:** Notarized (A) — reuses the existing `Relationship` DHT entry type (projection comment: *"Classification: A (Notarized)"*).
- **Kind:** `RELATES_TO` (the intersection of the three drifted vocabularies — sidesteps reconciliation).
- **Source of truth:** DHT `Relationship` → SQLite `relationships` projection. Coordinator/route: existing `content_store::create_relationship` + `POST /db/relationships` (seed) + `GET /db/relationships/graph/{id}` (read). **No new route, no new migration.**

### Entity: Computed Discovery Edge (tag co-occurrence)
- **Classification:** **Operational (C)** — recomputed-on-read from notarized content + `content_tags`; reconstructable on any peer; **never persisted, never anchored, never notarized.** The trait having no write method is the structural enforcement.
- **Source of truth:** none — it *is* a materialized view. SQLite-only at most (optional operational cache).

**Gate verdict:** clean. No new entry type, no new route, no new notarized field, no new DHT spend. No escalation.

## 4. Architecture — the Rust truth layer

### 4.1 The seam: `ContentGraphResolver`

New module `elohim/elohim-storage/src/graph_engine.rs` (sibling to `relationship_service.rs`; deliberately **not** named `GraphEngine` — `graph::engine::GraphEngine`, the EPR-projection CozoDB engine, already exists and is a different concern). `RelationshipService` becomes a *consumer* of the trait, not the owner.

```rust
// elohim/elohim-storage/src/graph_engine.rs

/// One edge in a resolved neighborhood. Composes BOTH explicit (stored,
/// notarized — Category A) and computed (recomputed-on-read — Category C)
/// edges into one uniform shape, discriminated by `inference_source`.
#[derive(Debug, Clone)]
pub struct ResolvedEdge {
    pub target_id: String,
    pub relationship_type: String,   // RELATES_TO for both classes in this slice
    pub confidence: f64,
    pub inference_source: String,    // "explicit" (A) | "tag" (C). Never persisted for C.
    pub depth: u32,                  // 1 direct; >1 transitively-reached explicit edges
}

#[derive(Debug, Clone)]
pub struct ResolvedNeighborhood {
    pub root_id: String,
    pub edges: Vec<ResolvedEdge>,
}

/// Bounded knobs for one resolution. Honors graph.json edge/index intent.
#[derive(Debug, Clone)]
pub struct GraphQuery<'a> {
    pub root_id: &'a str,
    pub max_depth: u32,                          // explicit-traversal bound (default 2, cap 3)
    pub relationship_types: Option<&'a [String]>,
    pub include_computed: bool,                  // gate the Category C pass
    pub max_computed: usize,                     // cap discovered edges (default ~25)
    pub min_shared_tags: usize,                  // tag-overlap threshold (default 1)
}

/// The seam. A future Cozo/datalog/embedding engine is just another impl —
/// never a rewrite of callers. Read-only: this trait NEVER writes edges.
pub trait ContentGraphResolver: Send + Sync {
    fn resolve_neighborhood(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<ResolvedNeighborhood, StorageError>;
}
```

### 4.2 `NativeGraphResolver` — two-pass composition

```rust
pub struct NativeGraphResolver { pool: DbPool }

impl ContentGraphResolver for NativeGraphResolver {
    fn resolve_neighborhood(&self, ctx, query) -> Result<ResolvedNeighborhood, StorageError> {
        // PASS 1 — EXPLICIT (Category A): depth-bounded BFS over stored relationships,
        //   reusing relationships_diesel::get_outgoing_relationships per frontier node,
        //   visited-set for cycles (the existing would_create_cycle BFS is the template).
        //   Each edge carries its stored inference_source ("explicit") + depth marker.
        // PASS 2 — COMPUTED (Category C), only if include_computed:
        //   content_tags self-join (§4.3), exclude root + any target already in Pass 1
        //   (explicit precedence). inference_source = "tag", confidence = overlap heuristic,
        //   depth = 1. Cap at max_computed.
        // Merge with explicit precedence on duplicate target_id.
    }
}
```

`RelationshipService` keeps its public `get_graph`/`get_graph_with_depth` signatures (so `http.rs` and tests don't churn), holds an `Arc<dyn ContentGraphResolver>`, delegates, and maps `ResolvedNeighborhood` → the wire `ContentGraphView`.

### 4.3 Computed discovery — tag co-occurrence (the MVP signal)

Tags are a **normalized join table** `content_tags(h_app_id, content_id, tag)` — no schema work. The overlap query (the cheapest feasible discovery):

```sql
SELECT ct2.content_id, COUNT(*) AS shared
FROM content_tags ct1
JOIN content_tags ct2
  ON ct1.tag = ct2.tag
 AND ct1.h_app_id = ct2.h_app_id
 AND ct2.content_id <> ct1.content_id
WHERE ct1.h_app_id = ?app
  AND ct1.content_id = ?root
GROUP BY ct2.content_id
HAVING shared >= ?min_shared_tags
ORDER BY shared DESC
LIMIT ?max_computed;
```

`confidence = shared / max(tags(root), 1)` clamped to `[0,1]` — bounded, recomputable, no persisted magic constants. Diesel via `sql_query` + `QueryableByName`, or a self-aliased `content_tags::table`.

**Why tag-overlap is the right first signal:** it is *genuine* discovery — it surfaces nodes with **no authored edge** (the whole point of Category C), whereas depth>1 only re-walks explicit edges. Depth>1 still ships (it's the `max_depth` bound on Pass 1 and finally de-stubs `get_graph_with_depth`), but the *discovery* story rests on the tag pass.

### 4.4 The A/C seam invariant (load-bearing)

Computed edges are returned in the response **only**. They are never passed to `create_relationship`/`bulk_create`, never written to `relationships`, never given a `dht_anchor_hash`. The trait has **no write method** — it *cannot* persist. Recompute-on-read is the default; an optional in-process LRU keyed by `(root_id, query-knobs)` is a later operational optimization, never a notarized store. Because both passes read only local SQLite, resolution works **offline, no doorway, no DHT round-trip** — and two peers with the same content compute the same tag edges with no consensus. That is *why* discovery is Category C.

### 4.5 `graph.json` honored as the declarative model

A `GraphSpec` loader parses `graph.json` once at startup into: (a) the **edge-type vocabulary** → the Pass-1 traversal whitelist; (b) **index intent** (`prereq_forward`, …) → optional matching SQLite indexes on `relationships(h_app_id, source_id, relationship_type)` + frontier ordering; (c) **rules** → the trait's future `resolve_rule` surface. `prerequisite_chain` (transitive closure to `max_depth`) is *already* what Pass-1 BFS computes — the native impl realizes the simplest rule today; the Cozo impl realizes all of them behind the same trait. `mastery_frontier` is explicitly deferred.

### 4.6 Wire contract — promote to ts-rs view, schema-first

`ContentGraph`/`ContentGraphNode` move from `relationship_service.rs` into the ts-rs-anchored `elohim-views` crate, gaining `#[derive(Serialize, TS)]` + `#[serde(rename_all = "camelCase")]` + `#[ts(export, export_to=…/generated/)]`. `ContentGraphNode` gains **`inference_source`** (→ `inferenceSource`) and `depth`; keeps `content_id`→`contentId`, `relationship_type`→`relationshipType`, `confidence`, `children`.

Follow the "adding a new view" checklist (root CLAUDE.md): (1) write `elohim/sdk/schemas/v1/views/content-graph.schema.json` **first** (camelCase; `inferenceSource` enum `["explicit","tag"]` for this slice; `depth`); (2) match Rust structs in `elohim-views`; (3) add a `schema_contract.rs` case; (4) add `content-graph` to `INTERFACE_FILES` in `codegen-ts.mjs`; (5) `cargo test export_bindings` + `pnpm run schema:codegen:ts`; (6) pre-push validates freshness. **Cross-crate ts-rs caution:** the struct move is cross-crate — do it atomically, build `--workspace`, verify generated TS via codegen (not per-crate), guard the silent-`From`-drop trap. Promoting this un-contracted wire **fixes the latent `root_id`/`rootId` field-drop bug** as a side effect — note it in the commit so the honesty matrix reflects the wire was un-contracted before.

### 4.7 `inference_source` vocabulary — one home (the discipline cut)

Canonicalize on the DHT/storage vocabulary (`explicit | path | tag | semantic`, `+system` operational). The DHT entry is the source of truth; snake_case never leaves Rust; types flow Rust→TS generated. **Retire the hand-written lamad TS enum** (`author|structural|semantic|usage|citation|system`) in favor of the generated vocabulary. The UI groups via a single `isDiscovered(inferenceSource)` predicate (`explicit` → authored; `tag`/`path`/`semantic` → discovered) — a predicate, not a wire reshape. This is a small, central, bounded reconciliation; the larger relationship-*kind* drift stays out of scope.

## 5. Architecture — the Angular surface

### 5.1 `ExplorationSidebarComponent` (shared)

`app/lamad/src/app/components/exploration-sidebar/` — standalone, OnPush, a **pure compositional wrapper** over the already-viewer-agnostic `mini-graph` + `related-concepts-panel` + explore button. Owns no data fetching; children self-fetch from `contentId`/`focusNodeId`. UX-surface only — shapes *how* relations show, never *what is true*.

```
@Input({ required: true }) contentId: string;
@Input() collapsible = true;          // lesson-view: off-canvas; viewer: pinned
@Input() open = false;                // [(open)] two-way
@Input() compact = true;
@Input() relatedLimit = 4;
@Input() graphDepth = 1;
@Input() graphHeight = 180;
@Output() exploreContent = new EventEmitter<string>();
@Output() exploreInGraph = new EventEmitter<void>();
@Output() openChange = new EventEmitter<boolean>();
```

### 5.2 Both viewers

- **LessonViewComponent**: delete its inline `<aside class="exploration-panel">` (lines 209–264) + toggle/backdrop + panel CSS; drop in `<app-exploration-sidebar [contentId]="content.id" [collapsible]="true" [(open)]="…">`, preserving its existing `exploreContent`/`exploreInGraph` outputs to PathNavigator.
- **ContentViewerComponent**: add `<app-exploration-sidebar [contentId]="node.id" [collapsible]="false">` to the content tab; wire `(exploreContent)` to its existing graph-node handler and `(exploreInGraph)` to `exploreInGraph()` (ts:1289).

### 5.3 Reconcile content-viewer's three relation surfaces

- **Retire** the inline "Related Content" grid (html:400–419, `loadRelatedNodes` ts:990–1011) — the sidebar's `related-concepts-panel` supersedes its `relatedNodeIds` subset. (Affinity-% badge lost → backlog.)
- **De-dup** the Network-tab mini-graph (html:802–829): remove the duplicate mini-graph + explore button; keep the Network tab for resilience/shard/attestation material.
- **Keep** `<app-epr-relationships-panel>` (html:429–438) exactly where it is — a distinct protocol-EPR-Head surface with its own `epr-link-navigation.feature` a2o coverage. Not folded in this slice.

### 5.4 Explicit-vs-discovered rendering

Close the dropped-`inferenceSource` plumbing: add `inferenceSource` to the `ContentRelationship` model + `DataLoaderService.transformToContentRelationship` (one-field pass-through; the wire already supplies it). In `RelatedConceptsService.categorizeRelationships`, route `isDiscovered()` edges into a new `discovered` bucket. Add a **"Discovered — you might also explore"** section to `related-concepts-panel` (muted/dashed, `data-testid="discovered-concept-card"`, an `inference-source` attribute/badge "via tag"). Computed edges read unmistakably as discovery; authored sections stay authoritative. (Optional: dashed `.edge-line.discovered` variant in the mini-graph.)

### 5.5 Empty-trap mitigation

- Resolve **only** via the live `ContentBackendService` path (`/db/relationships`, `/db/relationships/graph/{id}`). Never route the sidebar through `projection-api.service` — its `getRelationships`/`getRelatedContent` are `of([])` stubs.
- Populated-panel guarantee rests on `related-concepts-panel`'s **per-node** `getRelationshipsForNode(contentId)` (robust for off-path commons docs).
- **Root the mini-graph at the actual content id**, not the hardcoded `'manifesto'` (`data-loader.service.ts:1504`) — the Rust `{id}` graph route supports it. Small Angular change; the per-node panel is the guarantee, the mini-graph is best-effort.

## 6. Content / seeding (the first witness)

Hand-author three new `genesis/data/lamad/content/{constitution,confession,theology}.json` mirroring `manifesto.json`:
- `contentFormat: "markdown"` (renders via the deployed MarkdownRendererComponent — **independent** of the absent epr-composite/PathViewer keystone).
- `reach: "commons"` (intent; the seeder stores `public` — both pass doorway's anon gate, ungated either way).
- `contentType`: `reference` (or `epic` to match manifesto) — a doctrinal essay.
- **Not** added to any `genesis/data/lamad/paths/*.json` `conceptIds`/`resourceId` — standalone, addressable, out of the default path.
- `relatedNodeIds` on all four mirroring the cites mesh (each → the other three). The normal seed flow (`seed.ts:1341–1391`) turns these into `RELATES_TO` `inference_source='explicit'` rows — confirmed end-to-end.

Validate + seed: `pnpm run validate` then `npx tsx src/seed.ts --content-only --ids=constitution,confession,theology` (idempotent upsert). Shareable at `doorway.elohim.host/epr/{slug}`.

## 7. Verification — story-first a2o

Page model: `genesis/a2o/src/framework/pages/epr-content.page.ts`. New testids land **in the same commit** as the components (testid-sync): `exploration-sidebar`, `exploration-explore-graph` (one shared id replacing `lesson-explore-graph`/`viewer-explore-graph`), `related-concept-card`, `discovered-concept-card` + `inference-source`.

```gherkin
Feature: Shared exploration sidebar surfaces authored and discovered neighbors

  Background:
    Given "manifesto","constitution","confession","theology" are seeded as commons markdown EPR nodes
    And "confession" has authored relationships to "constitution","manifesto","theology"
    And the backend computes at least one discovered (tag) neighbor for "confession" not in its relatedNodeIds

  Scenario: Standalone viewer shows the See-also sidebar with both edge kinds
    When the learner opens "/epr/confession"
    Then the "exploration-sidebar" is visible
    And it lists "constitution","manifesto","theology" as authored related concepts
    And it shows at least one "discovered-concept-card"
    And the discovered card is labelled as discovery (inference-source not "explicit")

  Scenario: The same sidebar appears inside the path-step lesson view
    Given the learner is on a path step rendering "confession"
    Then the "exploration-sidebar" is visible with the same authored + discovered neighbors

  Scenario: Doctrinal markdown renders independently of the path keystone
    When the learner opens "/epr/theology"
    Then the markdown content body is rendered
    And the "exploration-sidebar" is populated
```

## 8. Implementation outline (ordered, bounded)

**Rust truth layer**
1. Write `content-graph.schema.json` (schema-first; camelCase; `inferenceSource` ∈ {explicit, tag}; `depth`).
2. New module `graph_engine.rs`: `ContentGraphResolver`, `ResolvedEdge`, `ResolvedNeighborhood`, `GraphQuery`.
3. `NativeGraphResolver` Pass 1 (explicit depth-bounded BFS) — de-stubs `get_graph_with_depth`.
4. `NativeGraphResolver` Pass 2 (tag co-occurrence, §4.3) — read-only, gated, capped, explicit-precedence merge.
5. `GraphSpec` loader — parse `graph.json` into traversal whitelist + index/rule intent.
6. Promote `ContentGraph`/`ContentGraphNode` to `elohim-views` ts-rs (+`inferenceSource`,`depth`, camelCase); `views_convert` converter.
7. Canonicalize `inference_source` enum (schema/codegen); retire the lamad TS drift.
8. Wire `RelationshipService` to `Arc<dyn ContentGraphResolver>`; map to view.
9. Route delta: `handle_db_content_graph` parses `depth|computed|minSharedTags|maxComputed`.
10. Contract + codegen: `schema_contract.rs` case, `INTERFACE_FILES`, `export_bindings` + `schema:codegen:ts`.
11. Resolver unit tests (in-memory SQLite): explicit depth-1, explicit depth-2, tag discovery, explicit-precedence, `max_computed` bound, `include_computed=false` → zero C edges.

**Angular**
12. `ExplorationSidebarComponent` (+ spec).
13. Edit `lesson-view` (extract aside → shared component).
14. Edit `content-viewer` (add sidebar; retire inline grid; de-dup network mini-graph).
15. `inferenceSource` plumbing: `ContentRelationship` model + `DataLoaderService` + `RelatedConceptsResult.discovered` + `related-concepts-panel` "Discovered" section.
16. Root mini-graph at actual content id.

**Content + story**
17. Author 3 markdown EPR seed JSONs + `relatedNodeIds` on all four; validate + seed.
18. testids → `epr-content.page.ts`; a2o feature (§7) committed with the code.

## 9. Risks & open coordination (no orphans)

- **Vocabulary drift (coordination):** the lamad `RelationshipInferenceSource` vs DHT/storage vocab — §4.7 canonicalizes `inference_source` here; the broader relationship-*kind* drift (3 vocabularies) stays a tracked follow-up, same class as the Reach-enum drift.
- **Local-stack DHT-anchor gap:** the relationship read path does **not** filter on `dht_anchor_hash` (consistent with `validate_content_exists(require_provenance=false)`), so the graph works on a freshly-seeded local stack pre-anchoring. **Keep the relationship read provenance-open** for offline correctness — do not add a provenance gate there.
- **Mini-graph rooting:** manifesto-rooted `getGraph()` won't reach off-path commons docs; §5.5 roots it at the content id. The per-node panel is the populated-panel guarantee.
- **Affinity-% badge** lost when the inline grid is retired → backlog card.
- **EPR-Slice-1 deploy caveat:** the absent PathViewer/epr-composite keystone (`ci-genesis-epr-lens-composite-outline-not-deployed.md`) is the path-*outline* renderer; our docs are `markdown` → unaffected. Integrator owns the clean app rebuild/redeploy.

## 10. Key files

| Concern | File |
|---|---|
| Resolver seam (new) | `elohim/elohim-storage/src/graph_engine.rs` |
| Service consumer + view structs to promote | `elohim/elohim-storage/src/services/relationship_service.rs` |
| Explicit-edge reads (Pass 1) | `elohim/elohim-storage/src/db/relationships_diesel.rs` |
| Tags (Pass 2) | `elohim/elohim-storage/src/db/content_diesel.rs`, `db/diesel_schema.rs` (`content_tags`) |
| Route to extend | `elohim/elohim-storage/src/http.rs` (`handle_db_content_graph`) |
| ts-rs view home | `elohim/elohim-views/src/` |
| Wire schema (new) | `elohim/sdk/schemas/v1/views/content-graph.schema.json` |
| Codegen / contract | `elohim/sdk/schemas/scripts/codegen-ts.mjs`, `elohim/elohim-storage/tests/schema_contract.rs` |
| Declarative model | `elohim/sdk/domains/lamad/manifest/graph.json` |
| EPR-projection engine (do NOT conflate) | `elohim/elohim-storage/src/graph/engine.rs` |
| Shared sidebar (new) | `app/lamad/src/app/components/exploration-sidebar/` |
| Viewers | `app/lamad/src/app/components/{lesson-view,content-viewer}/` |
| Panel + service | `app/lamad/src/app/components/related-concepts-panel/`, `services/related-concepts.service.ts` |
| inference_source plumbing | `app/lamad/src/app/services/data-loader.service.ts`, `models/content-node.model.ts` |
| Seed content (new) | `genesis/data/lamad/content/{constitution,confession,theology}.json` |
| Seed flow (explicit edges) | `genesis/seeder/src/seed.ts` (1341–1391) |
| Page model + feature | `genesis/a2o/src/framework/pages/epr-content.page.ts`, `genesis/a2o/features/content/` |
