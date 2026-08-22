---
index: false
name: project_automerge_content_sync_plane_lit
title: Automerge content-sync plane LIT
description: "Automerge storage-sync plane LIT (producer + libp2p convergence proof); producer MUST write h_app_id=\"elohim\"; back-fill default-ON w/ fleet-safety invariants."
metadata: 
  node_type: memory
  type: project
  originSessionId: b43fc98d-a01d-42eb-a935-abbc1f66659f
---

The `/elohim/storage-sync/1.0.0` Automerge engine in `elohim/elohim-storage` was fully wired
end-to-end (60s scheduler → ListDocuments → SyncChanges → apply_changes → sled) but **inert**:
nothing projected content into the Automerge DocStore, so every sync round round-tripped empty.
**Lit 2026-06-27** (commits `5606c6b77`..`64a02a3cb` on `feat/frontend-eyes-sprint`, commit-only):

- **The missing organ = a producer.** `src/sync/projector.rs`: an EventBus listener
  (`spawn_content_projection_listener`, mirrors `spawn_logging_listener`) reacts to
  `ContentCreated`/`ContentUpdated`, re-SELECTs the full row, and calls `project_content_doc`
  → one Automerge doc per content id, `doc_id="node:{id}"`. Wired at `main.rs` `Services::new`
  co-scope (~:2574), NOT :2611 — that's the one place `services.events`+`pool`+`p2p_node` are live.
- **LOAD-BEARING namespace:** the producer MUST write under `h_app_id="elohim"`.
  `initiate_sync_round` hardcodes `h_app_id:"elohim"` (`p2p/mod.rs:6996`) — a doc under any other
  ns (e.g. "lamad") sits inert forever. Guarded by `PROJECTION_NAMESPACE` const + a coupling test.
  (`h_app_id` on the sync plane is a sync-partition label, NOT the DNA app id.)
- **API realities (the plan's guesses were wrong):** there is **no `DocType` enum**
  (`infer_doc_type` is a `&self -> String`, added `"content"` arm); **no `SyncManager::save`** —
  the real mutate+persist idiom is `apply_changes(ns, doc_id, vec![doc.save()])` (same path a
  peer's changes take); added `SyncManager::get_doc_field` accessor (`get_heads` existed).
- **Proof:** `tests/sync_libp2p_convergence.rs::doc_authored_on_a_converges_to_b` — two real
  `SyncManager`s on temp sleds joined by a real libp2p swarm; B converges `node:edit-prop-1`
  (`title=="Edited v2"`, `heads_a==heads_b`). Has teeth (RED 30.35s deadline w/o apply_changes,
  GREEN 0.35s). Closes the zero-integration-coverage gap on the libp2p sync leg
  (`sync_integration.rs` only proved merge logic). Independently re-verified.
- **Back-fill default-ON since 2026-07-02** (commit `f4e713ff2` on feat/frontend-eyes-sprint,
  commit-only; supersedes "go-forward only"): every cold start runs the idempotent corpus back-fill
  (`ELOHIM_DOCSTORE_BACKFILL=0` is the explicit opt-out; iroh-mode peers still get NO back-fill).
  Fleet-safety invariants added the same day (adversarial-review MAJORs — do NOT regress):
  (1) **empty-never-projects** — unknown serving/nullable fields are ABSENT from the projection,
  never ""/0 (empty can WIN the LWW merge and poison the fleet); `updatedAt` is never projected
  (restart ping-pong history inflation). (2) **reconcile-offers, events-assert** — back-fill uses
  `project_content_doc_reconcile` (fills absent/empty doc fields, never contests non-empty ones);
  only fresh event writes assert. (3) **fail-closed reach gate** — ONLY broadcast tiers
  (`community|public|commons`) enter the sync plane (`reach_is_distribution_safe`); private/scoped
  AND unknown-vocab values excluded (plane has no receiver-side reach auth; doc carries full body).
  Content that must sync MUST be broadcast-tier (create default is `public` — e2e unaffected;
  test fixtures needed `commons`, "household" is unknown-vocab = excluded). (4) **amber converges
  set→set** — heal replaces non-green blob_hash; green (`dht_anchor_hash`) inviolable (the old
  write-iff-empty froze rows on their first heal). (5) sync rounds PAGINATE (cursor map +
  `MAX_SYNC_LIST_OFFSET` cap) — pre-fix, docs past the first 1000 silently never synced.
  This CLOSED the elohim.host landing-404 live (verified 2026-07-02, edge #1135: 200 + blobHash
  converged from matthew, trust=published — the replication plane raced the CRDT heal and won;
  federation-deploy + blob-replication + epr-projection-fallback + peer-mesh RED suites all flipped
  green). Gotcha shipped with it: a NEW stage pushed the edge Jenkinsfile past the CPS 64KB cliff at
  19 stages (#1134, count-gate said fine) — stages-model cost is TOKENS not stage count; hook
  recalibrated with labeled data + wired into lint-jenkinsfiles-fast.sh phase 1.
- **Stretch + iroh ALSO LANDED 2026-06-27** (operator "carry on"; commits `e4fb14727` iroh,
  `f37b28509` doorway, `9357b10df` frontend):
  - **iroh-mode producer** wired (`p2p-iroh` feature). BUT iroh content won't FLOW yet — the 60s
    `initiate_sync_round` driver is libp2p-only; iroh fills the DocStore but has no periodic round
    driver (`IrohSyncClient` is test/bench-only). Backlog `iroh-sync-round-driver-gap.md`.
  - **doorway `/sync` carriage** — storage `/sync/v1` handlers already existed (`http.rs:981`); just
    declared them in `build_manifest` (FLIPPED the `test_manifest_builds` guard that forbade /sync)
    + doorway `is_service_path` owns `/sync` + fixture + unit test. SECURITY-ADJACENT: `/sync` now
    exposed through doorway — GET reads unauthenticated (inherit `http-reach-enforcement-gap`), POST
    `/changes` auth_required. No browser caller yet.
  - **frontend ESM** — smoke-test caught the plan's "wasm base64-inlined" premise as FALSE (automerge
    3.2.4 browser export breaks Zone.js Angular builds); fix = tsconfig alias to automerge's
    `fullfat_base64.js` (lazy chunk, no budget hit). SDK flipped to ESM (no consumer breakage).
    `ContentDocSyncService` added (uninjected → tree-shaken until a component consumes it). CAVEAT:
    alias hardcodes an internal automerge dist path (brittle across upgrades; types→any in app).
  - NOTE: `pnpm run build` for elohim-app fails on a PRE-EXISTING env gap (`build:wasm` →
    `wasm-pack: command not found`); verify the app via `pnpm exec ng build` (bypasses the prebuild).
  - **G7 card surfacing + browser-leg proof LANDED 2026-06-27** (commit `1b70c0532`; recovered after a
    mid-session agent crash — work was in the worktree uncommitted, build-verified then committed).
    `ContentDocSyncService` promoted to `@elohim/service` (shared lib feeds lamad card + elohim-app);
    client-composed "Doc sync" sibling row on `<elohim-resilience-snapshot>` (facings spec §12 — NOT a
    server fold: plane separation + determinism); lamad content-viewer consumer; `/sync` added to dev
    proxies; `/dev/doc-sync` look-rail harness. Builds green in all 3 contexts; card 26/26, service 5/5,
    harness 2/2. **Browser leg PROVEN** via look rail: prod `dist` + a wire-faithful fake `/sync` (real
    automerge change) → real AutomergeSync fetched→applied→rendered a converged doc (Status: synced).
  - **GOTCHA — automerge base64 alias works in prod build but NOT under `ng serve`/`pnpm start`**: the
    Angular dev server pre-bundles automerge via package `exports` → the wasm-bindgen BUNDLER entry →
    `automerge_wasm_bg.wasm` 500 (doc-sync silently broken in local dev; prod unaffected). The look rail
    caught it. Backlog `dev-serve-automerge-wasm-bundler-entry-gap.md`. Prod runtime is clean (base64
    inlined, no .wasm fetch).

Plan: `genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md`
(D5 data plane). This is the Automerge/libp2p storage plane — DISTINCT from the Holochain DHT
content plane (which already syncs: `content_visible_across_agents` passes; edit-propagation
sweettest backlogged) and from blob/shard custody. See
[[project_inventory_exchange_not_byte_replication]], [[project_dataplane_next_lens_diversity_placement]],
[[project_principle_p1_reconciliation_controller]].
