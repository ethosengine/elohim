---
name: project_content_graph_native_rust_not_cozo_apollo
description: Content relationship graph is owned in native Rust storage services for mission-critical perf — NOT Cozo/Kuzu/Apollo; graph.json CozoScript is aspirational spec only; the ContentGraphResolver trait seam is being introduced 2026-06-08
metadata: 
  node_type: memory
  type: project
  originSessionId: c24e8f16-ba80-4a07-90b6-9bf68242bcd9
---

The lamad content **relationship graph** (content↔content edges) is deliberately computed in the **lower-level Rust storage services**, NOT delegated to an external/embedded graph engine. Operator rationale (2026-06-08, not derivable from code): *"pushed hard for lower-level graph coherence in the rust-side services because we want mission-critical graph performance."*

**Lineage / non-adoptions (verified by repo search):**
- **Kuzu** (embedded property-graph DB) was explored and **deprecated** — `"replaces Kuzu"` comments at `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:856,3888`. Relationships migrated to Holochain DHT links + a SQLite `relationships` projection.
- **Cozo** is never a dependency and never wired. Its only trace is `elohim/sdk/domains/lamad/manifest/graph.json`, whose `rules` are written in **CozoScript datalog** (`:=`, `*epr_edge{...}`) — an **aspirational declarative spec no runtime reads or executes** (the datalog rules `prerequisite_chain`/`mastery_frontier` are consumed by nothing).
- **Apollo** appears nowhere — wrong layer (centralized GraphQL server vs P2P/local-first + native perf).

**Abstraction state (as of 2026-06-08):**
- Engine-AGNOSTIC at the data+spec layer: DHT `Relationship` entry + `inference_source` enum (`explicit|path|tag|semantic`, `INFERENCE_SOURCES[4]`; storage `valid_sources` adds `system`) + the `graph.json` edge/node/index/rule model.
- There was **NO graph-engine trait**: `RelationshipService::get_graph` (relationship_service.rs) is concretely diesel-bound, **hardcoded depth-1** (*"The Diesel module doesn't have a graph traversal function"*); `get_graph_with_depth` is a stub. The codebase HAS the trait-culture to add one (`InferenceEngine` mock/local/remote, `EprStore`, `TallyStrategy`, many `*Backend` traits).
- **The `feat/native-content-graph-seam` slice introduces `trait ContentGraphResolver`** (`graph_engine.rs`) as that seam — native two-pass impl (explicit BFS + tag-overlap discovery); a future Cozo/datalog/embedding engine becomes one more impl behind it. Spec: `native-content-graph-seam-design`; plan: `native-content-graph-seam`.

**How to apply:** Seeded explicit author edges = Category A (notarized); computed/inferred edges = Category C (recompute-on-read, NEVER persisted/anchored). Read route is `GET /db/relationships/graph/{id}`. Use `RELATES_TO` (intersection of the 3 drifted relationship-kind vocabularies — manifest 11 / DHT 6 / storage 16) to avoid the drift. `inference_source` canonical home is the DHT vocabulary; the lamad TS `RelationshipInferenceSource` enum is drift. Don't reach for Cozo/Apollo — extend the native trait. See [[project_reach_enum_drift_reconciliation]] for the sibling vocab-drift pattern.
