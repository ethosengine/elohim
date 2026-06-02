---
status: Design
---

# Sprint Kickoff — Elohim SDK + EPR-App Boundary Cleanup, Reliability & Classification

> Seed brief for a fresh session. NOT a finished spec — the sprint should START with
> `superpowers:brainstorming` to shape the SDK contract + app-manifest, then write the spec
> (`genesis/docs/superpowers/specs/`) and plan (`genesis/docs/plans/`) before implementing.

## Mission

Turn the implicit, leaky "build an app on the substrate" path into a clean, documented,
reliable **two-tier system**:

1. **elohim-core** — the substrate + the **Elohim SDK**: the contract anyone (us OR a third
   party) builds an EPR-app against.
2. **Reference EPR-apps** — the protocol's own apps, each proving the contract end-to-end:
   - **Elohim Protocol landing** — a nascent **CMS**
   - **Lamad** — the **LMS**
   - **Shefa** — the **mint / drive**
   - **Avodah** — the **work pipeline**
   - **Mishpat** — the **governance / policy** system
   - **Qahal** — the **social network**

The bar: a third party can build, manifest, seed, project, and serve their own EPR-app on
this substrate **without touching substrate code** — and it loads fast and reliably.

## Why now — the evidence (read this; it's the whole motivation)

The 2026-05-30 overnight shift took a **7-layer cascade** to get even ONE doorway's pages to
render. Every layer was the *same class* of defect — the EPR-app **build → bundle → seed →
project → serve → render** path is informal, so each consumer reinvented a corner and a
different corner broke:

1. storage `/apps` content-format filter excluded `spa-bundle` (+ indexed by inner slug not id)
2. `extract_app_context` routing-shadow: `/db/rea_commitments` fell through to `handle_db_stats` (dead-code handler)
3. `in_scope_of = NULL` (camelCase-deser drift) → projection query returns `[]` → empty router
4. `upsert_with_anchor` didn't backfill projected columns → reseed couldn't self-heal
5. `stageSpaBlobs` seeded a single doorway (apex backend never got bytes/blobHash)
6. apex `DOORWAY_ID` vs seed-scope drift; EprRouter never self-healed (one-shot boot-fetch)
7. **the standalone bundles white-paged on bootstrap** — missing DI providers
   (`LAMAD_HOLOCHAIN_CLIENT`, `ELOHIM_CLIENT`, `BLOB_FETCHER`, `GOVERNANCE`,
   `CONTENT_ATTESTATION`, `ECONOMIC_EVENT_FACTORY`) because the projected-bundle bootstrap was
   never a real, SDK-owned surface (it was only ever exercised by the dev server).

And on the live load you observed the **`apps-sw` service-worker zip-unpack fallback firing**
— the client downloading + decompressing the whole app zip — instead of a blazing-fast hit
on a **warmed doorway projection cache**. The delivery fast-path is unreliable; the fallback
is masquerading as the default.

Full root-cause writeups to read first:
- `.claude/shifts/2026-05-30T03-15-landing-pages-both-doorways.sprint-result.md`
- `.claude/memory/project_epr_projection_serving_chain.md`
- `.claude/memory/project_alpha_edge_deploy_debugging_landmarks.md`

## System boundary inventory (current — verify + make canonical)

**Tier 1 — Notarized contract (DHT truth + protocol schema)**
- DNAs: `elohim/holochain/dna/{elohim,imagodei,mishpat,infrastructure,node-registry,hrea}` (`lamad-v1` archived). `elohim` DNA is the SDK/contract boundary ([[project_elohim_dna_as_sdk_boundary]]).
- Protocol Schema: `elohim/sdk/schemas/v1/` (enums, views, inputs, intents, commitments, **manifest/** incl. `app-manifest.schema.json`, `manifest-epr.schema.json`, `pillar-projection.schema.json`, dna-signals, p2p, registries).

**Tier 2 — The Elohim SDK (the surface third parties build against)**
- `elohim/sdk/` — schemas + `domains/{avodah,elohim,imagodei,infrastructure,lamad,mishpat,qahal,shefa}` per-pillar manifests + `storage-client-ts` (generated TS types from Rust views) + `epr-ts` + `src`.
- `elohim/epr/` — Rust EPR content-addressing module ([[project_first_class_graph_pattern]], epr-content-addressing skill).
- JS/Angular SDK: `app/elohim-library/projects/` — `elohim-service` (`@elohim/service`: `ElohimClient`, `provideElohimClient`, mode-aware browser/tauri/holochain client), `elohim-rea-runtime`, `elohim-identity`, renderer plugins (`html5-app-plugin`, `perseus-plugin`=sophia, `psephos-plugin`), `graphos` (design tokens), `lamad-ui`.
- `app/elohim-elements` — Lit protocol-native custom elements (e.g. `<elohim-default-omnibar>`).

**Tier 3 — Truth / runtime substrate**
- `elohim/elohim-storage` (Rust) — domain services, diesel persistence, dual P2P (libp2p + iroh), HTTP API (`views.rs` ts-rs boundary), `/apps/{epr_id}` bundle serving + slug_index, `rea_commitments`/project-epr.
- `steward/node` (Rust) — P2P runtime + desktop conductor embedding; `steward/device` (Tauri).
- `elohim/elohim-compute` — per-node compute reporting.
- `bridges/` — `valueflows` (atproto/activitypub planned): external-protocol → EPR-REA.

**Tier 4 — Web2 projection / gateway**
- `doorway/doorway-service` (Rust) — bootstrap, signal, conductor proxy, **manifest-driven route registry**, **EprRouter**, **projection cache** (MongoDB `AppFileCache` + `TieredBlobCache`, `projection/warm.rs` + `warm_stream.rs`), SSR renderer.
- `doorway/doorway-app` (Angular) — operator dashboard / `/threshold`.

**Tier 5 — EPR-app delivery boundary (THE flaky seam — primary target)**
- build → zip (`genesis/landing/scripts/build-and-zip.sh`, Angular app builds) → upload (`stageSpaBlobs` → `/admin/seed/blob` + `PATCH /db/content/{slug}`) → project-epr commitment (`genesis/seeder/src/seed-projections.ts`) → `EprRouter` → `dispatch_to_projected_epr` → storage `/apps/{epr_id}` → projection cache (warm) → client.
- **`apps-sw` service worker** (scope `/apps/`) zip-unpack **fallback** — fired instead of the warm-cache fast path. (Find its source — likely tied to `html5-app-plugin` / the bundle delivery; `seeder/blob-manager.ts` is the zip-creation side.)
- **`app-manifest.schema.json`** — the as-yet-underused contract for how an EPR-app declares its bundle / renderer / routes / projection scope / reach / doorway-agnostic config.

**Tier 6 — Reference EPR-apps (built ON the SDK)**
- `app/elohim-app` (Angular composable view federation — landing `elohim-host-landing` + pillar UIs), `app/lamad` (`lamad-spa`), `app/imagodei-portal` (`imagodei-portal`), `genesis/landing` (vanilla landing/CMS), `sophia` (assessment web component, submodule). Target end-state: the 6 pillar apps above as discrete EPR-apps.

**Tier 7 — Content + seeding**
- `genesis/` source content → `elohim-import` → seed-data JSON → `genesis/seeder` → storage; `seed-projections.ts`; `deployments.json` (seed-or-skip).

**Tier 8 — CI/CD pipeline boundary**
- `genesis/orchestrator` (graph-walker + per-project `build-manifest.json` + dispatch). Current pipelines: app, edge (holochain), dna, genesis, sophia, steward, doorway-app, elohim-library, epr, compute, orchestrator.

## Workstreams

### A — Harden the Elohim SDK (the core contract)
- Make `@elohim/service` + the schemas + `storage-client-ts` + the EPR/projection contract a **clean, documented, tested surface**. Define what "the SDK" *is* vs what's app-private.
- **Ship a STANDARD app-bootstrap** the SDK owns (e.g. `provideElohimApp(manifest)` / `bootstrapEprApp(...)`) so an EPR-app gets its DI providers (client, blob-fetcher, governance, attestation, event-factory, holochain client), same-origin doorway resolution, and SW/cache wiring **for free** — eliminating the per-app hand-wiring that white-paged last night. Reference apps adopt it.
- Audit every cross-pillar token + provider for "provided where consumed" across ALL bundle bootstraps (not just dev-server).

### B — Formalize the app-manifest + prove it
- Mature `app-manifest.schema.json` into the real contract: bundle identity (epr_id/slug), renderer, routes/url_path, projection scope (doorway-agnostic), reach, entry_file, base_href, cache policy, SW policy. (`p2p-design-gate` for any new entity/projection shapes.)
- Wire it so the build/seed/project pipeline and the SDK bootstrap are **driven by the manifest** (no hand-coded slugs/scopes/doorwayIds — kill the drift class from layers 5–6).
- Deliver ONE reference EPR-app that conforms end-to-end with zero substrate hand-wiring as the proof.

### C — Delivery reliability (the `apps-sw` vs warm-cache bug)
- Make the **warm doorway projection cache the reliable fast path**; the SW zip-unpack a genuine last-resort. Fix: cache warming (`warm.rs`/`warm_stream.rs` returned `{}`), the known **TieredBlobCache write-on-fetch TODO** (doorway/CLAUDE.md — blob tier never written on the proxy path), EprRouter population robustness (boot-fetch one-shot → self-heal already added; verify), and the apex-router/cross-peer projection leg (adam-side `in_scope_of`).
- Define + measure delivery SLOs (warm-hit %, time-to-render, no-SW-fallback) and **verify by render, not HTML shell** (last night's measure was a false-positive — it matched the static `<title>`).

### D — Classification + reuse
- Produce the **canonical boundary map** (ownership, contract, test coverage, "is this SDK / core / app-private?") for every component above.
- Remove the leaky duplication (per-app DI bootstrap, per-app doorwayUrl resolution, per-app cache/SW wiring) into SDK-provided primitives.

### E — Pipeline / monorepo split decision
- Evaluate splitting CI (and possibly the repo layout) into **elohim-core** (dna, storage, doorway, steward, epr, sdk, elohim-library, bridges) vs **reference-apps** (the 6 pillar EPR-apps), so the core contract versions independently and third parties consume a stable SDK. Decide + (if taken) execute the orchestrator/build-manifest reorganization.

## Approach
- **Brainstorm first** (`superpowers:brainstorming`) to shape the SDK contract + app-manifest + the split — this is design work, not a mechanical cleanup. Then `superpowers:writing-plans`.
- `p2p-design-gate` before any data-entity/projection design.
- Specialists: `rust-architect` (storage/doorway/epr/dna/compute/bridges), `angular-architect` (elohim-app/lamad/imagodei-portal bootstrap + SDK bootstrap), `component-architect` + `graphos-designer` (elements/library), `ci-investigator`/`ci-observer` (pipeline + orchestrator), `code-reviewer`/`pattern-hunter` (duplication + boundary leaks), `quality-architect` (test/coverage strategy for the contract).
- Begin with a **code-review + classification pass** (map every boundary + its contract + test coverage) BEFORE changing code — the classification is itself a deliverable.

## Deliverables
1. Canonical system-boundary map + classification doc.
2. Hardened, documented, tested Elohim SDK incl. the standard EPR-app bootstrap.
3. Formalized `app-manifest` + one reference EPR-app conforming end-to-end with no hand-wiring.
4. Reliable delivery: warm projection cache is the fast path; SW zip-unpack a true fallback; cache-warm + blob-tier-write + EprRouter population fixed; render-verified on both doorways.
5. Pipeline/monorepo split decision (+ execution if taken).

## Done
- A simulated third party builds + deploys an EPR-app against the documented SDK + app-manifest with **no substrate code changes**.
- Landing + Lamad load as **warm-cache fast hits (no `apps-sw` zip-unpack), verified by render**, on `alpha.elohim.host` AND `elohim.host`.
- Boundaries classified, contracts tested, duplication removed; the split decided.

## Constraints / context
- Read `CLAUDE.md` (build/test/gotchas), `doorway/CLAUDE.md`, `elohim/elohim-storage/CLAUDE.md`, `elohim/sdk/CLAUDE.md`, `app/elohim-library/CLAUDE.md`, and the 2026-05-30 sprint-result + memory entries above.
- Operator owns cluster ops (no kubectl from dev env); drive via code + CI (`[build:*]` tags). RUSTFLAGS gotcha (native vs WASM). sccache can serve a NUL-corrupted clippy probe → `RUSTC_WRAPPER="" cargo clippy` if it fires.
- The `sprint/cross-pillar-cleanup` branch carries the recent fixes; the **apex-router leg (adam-side projection of `apex-elohim-host` rows) may still be open** — confirm + close as part of Workstream C.
