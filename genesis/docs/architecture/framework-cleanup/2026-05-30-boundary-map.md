# Elohim Framework — Canonical System Boundary Map

> **Sprint 1 "Classify & Map" deliverable 1 of 3.** Read-only analysis; no code changed.
> Companions: [`core-vs-reference-split-decision.md`](./2026-05-30-core-vs-reference-split-decision.md) (deliverable 2),
> [`reliability-backlog.md`](./2026-05-30-reliability-backlog.md) (deliverable 3, scopes Sprint 2).
>
> **Inputs:** the [sprint sequence plan](../../plans/2026-05-30-elohim-framework-cleanup-sprint-sequence.md),
> the [boundary-brief kickoff](../../superpowers/specs/2026-05-30-elohim-sdk-epr-app-boundaries-sprint-kickoff.md),
> the [2026-05-30 landing-pages sprint-result](../../../../.claude/shifts/2026-05-30T03-15-landing-pages-both-doorways.sprint-result.md),
> memory `project_epr_projection_serving_chain` + `project_alpha_edge_deploy_debugging_landmarks`,
> the per-crate `CLAUDE.md` files.
>
> **Method:** an 8-agent read-only fan-out (rust-architect ×3, angular-architect ×2, ci-investigator,
> pattern-hunter, quality-architect) — one per tier-cluster plus two horizontal lenses (duplication,
> coverage). Highest-leverage claims spot-verified by the synthesizer against source.

## Why this map exists

The 2026-05-30 overnight shift took a **7-layer cascade** to get even one doorway's pages to render.
Every layer was the *same class* of defect: the EPR-app **build → bundle → seed → project → serve →
render** path is informal, so each consumer reinvented a corner and a different corner broke. You cannot
fix reliability without first knowing where the boundaries are, who owns each contract, and which seams
are untested. This document is that map. It classifies every component, flags every boundary leak and
duplication, and records the test coverage — feeding the split decision (deliverable 2) and the
reliability backlog (deliverable 3).

---

## The classification taxonomy (four buckets, not three)

The kickoff brief asked for a three-way classification — **SDK / elohim-core / app-private**. Analysis
(and an operator framing correction mid-sprint) surfaced a **fourth bucket** that the three-way scheme
was silently mis-filing: the **dev-tooling / delivery harness** (CI/CD + the seeder). Calling CI and the
seeder "core" or "app-private" is a category error — they are not a truth layer at all. They are
developer conveniences for **development, packaging, and delivery**, and *the very thickness of the
hand-coded delivery knowledge inside them is the symptom this whole cleanup is addressing.* They are
surfaced here as their own class so the map doesn't imply they hold contract-grade authority they don't.

| Class | What it is | The test | Where it lands in the repo split |
|---|---|---|---|
| **SDK** | The contract a **third party** builds an EPR-app against with *no substrate code changes*. | "Could this capability be captured at scale for rent-extraction?" → protocol primitive → SDK. | core repo (published surface) |
| **elohim-core (runtime substrate)** | Shared substrate the protocol *operates* but third parties reach only via the SDK/HTTP contract — they don't import it. | "Is this the engine, not the contract?" | core repo (internal) |
| **app-private (reference apps)** | Specific to ONE reference EPR-app (the pillar UIs and their bootstrap glue). | "Would a third party fork or replace this?" | reference-apps |
| **dev-tooling / delivery harness** | CI/CD orchestrator + per-project pipelines + the content seeder. **Not a truth layer** — packaging/delivery conveniences. | "Is this scaffolding that *should* be a thin manifest-driven consumer of the SDK, but currently hand-codes delivery knowledge?" | shared tooling (its thinness is the *goal*, see deliverable 2) |

**The cleanup's north star, stated as a classification rule:** every piece of delivery knowledge currently
living in the harness (slugs, scopes, doorwayIds, entry files, base hrefs, zip/hash logic, cache/SW
policy) is **mis-placed**. It belongs in the **app-manifest (SDK contract)**, consumed by an
**SDK-owned bootstrap** and a manifest-driven pipeline. The harness should get *thinner* as Sprints 3–4
land. Where this map flags harness logic, read it as "this is delivery knowledge that escaped the
contract," not "this is a layer with a bug."

---

## Tier-by-tier boundary map

Format per row: **Component** · *owner* · contract/interface · test coverage · **classification**.
Leaks and duplication are consolidated in the [register below](#boundary-leak--duplication-register).

### Tier 1 — Notarized contract (DHT truth + protocol schema)

| Component | Owner | Contract / interface | Coverage | Class |
|---|---|---|---|---|
| `elohim` DNA (`elohim/holochain/dna/elohim/`, **physically named `lamad`**, deployed as role `lamad`) | substrate | 75 entry types in `content_store_integrity`; coordinator `content_store`; the `EprProjectionView`/REA/Content/LearningPath/Manifest/FeedbackSignal contract | sweettest harness; **no entry-count headroom guard** | **SDK (contract)** |
| `imagodei` DNA | substrate | 25 entry types; Human/Agent/ContributorPresence/KeyStewardship/Collective/recovery | sweettest | **SDK (contract)** |
| `mishpat` DNA | substrate | 9 entry types; Precedent/GovernanceState/Commitment(compute-delegation)/ChallengeOutcome | sweettest + `bounds_validator_integration.rs` | **SDK (contract)** |
| `infrastructure` DNA | substrate | DoorwayRegistration/ContentServer/PeerStatus/StringAnchor | sweettest (thin) | **elohim-core** (topology; not third-party-facing) |
| `node-registry` DNA | substrate | NodeRegistration/Heartbeat/HealthAttestation/Custodian/Shard | sweettest (thin) | **elohim-core** |
| `hrea` DNA | external (h-REA) | binary not in git; role commented-out in `happ.yaml`; Wave-3 placeholder | none | **elohim-core** (blocked on upstream) |
| `lamad-v1` DNA | archived | `healing_exports.rs` only — v1→v2 migration surface | none | **elohim-core (archived)** |
| Protocol schema `elohim/sdk/schemas/v1/` | SDK | 37 enum + 89 view + 10 input + 6 manifest schemas; **authoritative wire contract** | `schema:test` (AJV), `schema:validate` (3500+ seeds), `schema:check-dna`, codegen freshness gate on pre-push | **SDK** |
| `app-manifest.schema.json` | SDK | vocabulary + 3-leg coupling + signalKinds + projections + graph + graduation + constitutionalRatios | `test-manifest-schema.mjs` | **SDK** — *but the delivery contract is absent (see [Key finding 1](#key-boundary-findings))* |
| `epr-projection-view.schema.json` + Rust `elohim/elohim-views/src/projection.rs:18` | SDK | the notarized projection wire shape (commitmentId/eprId/doorwayId/urlPath/mode/reach/baseHref/entryFile) | `schema_contract.rs` round-trip | **SDK** |

### Tier 2 — The Elohim SDK (the surface third parties build against)

**Rust SDK**

| Component | Owner | Contract | Coverage | Class |
|---|---|---|---|---|
| `elohim/sdk/storage-client-ts/` | SDK | 428 ts-rs-generated `.ts` types from `elohim-views`; the runtime HTTP wire types (`@elohim/storage-client`) | `cargo test export_bindings`; freshness gate | **SDK** |
| `elohim/sdk/epr-ts/` | SDK | EPR codec types (Coupling/Envelope/EprKind/Reach) from `elohim-epr` | `epr/tests/export_bindings.rs` | **SDK** |
| `elohim/elohim-views/` | SDK boundary (Rust) | the ts-rs-anchored View/InputView structs | per-domain ts-rs export tests; **no standalone unit tests** | **SDK** |
| `crates/elohim-sdk/` | SDK | consumer facade; mode-aware ContentClient | **no tests** | **SDK** |
| `crates/{doorway-client,elohim-storage-client}/` | SDK | Rust HTTP clients for doorway + storage sync API | thin/unknown | **SDK** |
| `elohim/epr/` | SDK | content-addressing primitive | `schema_contract.rs`, `export_bindings.rs` | **SDK** |
| `elohim/sdk/src/` (`ElohimSDK` class) | — | **a full Holochain conductor WebSocket client** (admin/app ws) | — | **MISCLASSIFIED → elohim-core** (see [Key finding 4](#key-boundary-findings)) |
| `elohim/sdk/domains/<pillar>/manifest.json` | app vocab | per-pillar content-type/format/renderer vocabulary | manifest validators | **app-private vocabulary living in the SDK tree** ([Key finding 5](#key-boundary-findings)) |
| `deny.toml` (repo root) | SDK enforcement | bans non-server crates from importing `elohim-storage` | `cargo deny` on pre-push | **SDK boundary enforcement** |

**JS / Angular SDK** (`app/elohim-library/projects/*`, `app/elohim-elements/*`)

| Component | Owner | Contract (key exports) | Coverage | Class |
|---|---|---|---|---|
| `@elohim/service` (elohim-service) | SDK | `ElohimClient`, `provideElohimClient`, `ELOHIM_CLIENT`, `ELOHIM_ENV`, `BLOB_FETCHER`, `GOVERNANCE`, `CONTENT_ATTESTATION`, `detectClientMode`, Doorway/Governance/Observation services | Vitest on most services; **client + angular-provider layer untested** | **SDK** |
| `@elohim/rea-runtime` | SDK | `EVENT_API`, `AGENT_CONTEXT`, `ECONOMIC_EVENT_FACTORY` + 8 shefa tokens; `EventService`, `AttentionTrackerService` (fail-fast `providedIn:'root'`) | Vitest on 3 core services | **SDK** |
| `@elohim/identity` | SDK (partial) | session/attestation models; **identity/profile/guard exports PENDING** (entangled with `@app/imagodei`/`@app/lamad`) | 1 service spec | **SDK-in-progress** |
| `@elohim/perseus-plugin`, `@elohim/psephos-plugin` | SDK | UMD renderer plugins (sophia quiz, governance ballot) | **no specs**; perseus emits console.logs on load | **SDK (renderer plugins)** |
| `html5-app-plugin` | SDK | **type-only** html5-app/SW contract types; no package.json, no default provider | none | **elohim-core (types)** — impl lives app-side ([Key finding 2](#key-boundary-findings)) |
| `app/elohim-elements/elohim-core` | SDK | Lit primitives (`<elohim-button>`, `<elohim-default-omnibar>`, capability-profile system) | **28 specs — deepest element coverage** | **SDK** |
| `app/elohim-elements/elohim-imagodei` | SDK | login/consent/portal-shell/oauth-callback Lit elements | 18 specs | **SDK** |
| `app/elohim-elements/{elohim-shell,lamad,shefa,doorway,avodah}` | SDK | per-pillar Lit elements | **zero specs each** | **SDK (untested)** |
| `graphos` | tooling | Storybook-only; **no package.json, no runtime exports** | fixture specs | **dev-tooling (design system)** |
| `lamad-ui` | app | lamad pitch-surface diagram components; **no `@elohim/` scope** | **zero specs** | **app-private** |

### Tier 3 — Truth / runtime substrate

| Component | Owner | Contract | Coverage | Class |
|---|---|---|---|---|
| `elohim/elohim-storage` | substrate | the HTTP API (`http.rs`+`views.rs`, camelCase wire), libp2p+iroh planes, diesel SQLite projection, `/apps/{epr_id}` serving + slug_index, `/db/rea_commitments?action=project-epr`, `/manifest` self-registration | 139 integration + inline `#[test]`; **gaps:** no `extract_app_context` routing-shadow regression, no spa-bundle slug_index test, no `upsert_with_anchor` backfill test | **elohim-core** |
| `elohim/elohim-compute` | substrate | `ResourceReporter`/`HealthReporter` traits; per-node only ([[project_node_metrics_vs_hub_aggregation_boundary]]) | inline roundtrip tests | **elohim-core** |
| `steward/node` | P2P runtime | `elohim-node` binary; libp2p 0.54 custom protocols; Automerge sync; consumes elohim-storage as a `p2p`-feature library | config/sync/transport tests; **no dual-plane integration test** | **elohim-core** |
| `steward/device` | desktop shell | Tauri 2.x; runs elohim-storage as localhost:8090 sidecar | thin IPC tests; **no sidecar e2e** | **app-private** (one deployment shape of the steward archetype) |
| `bridges/valueflows` | bridge | VF-GraphQL at `/api/v1/vf-graphql`; M1 fixture-only | `m1_tracer_bullet.rs`, `m1_http_smoke.rs` | **elohim-core** |
| `bridges/{atproto,activitypub}` | bridge | planned | none | **elohim-core (planned)** |

### Tier 4 — Web2 projection / gateway (doorway)

| Component | Owner | Contract | Coverage | Class |
|---|---|---|---|---|
| `doorway-service` `EprRouter` (`projection/epr_router.rs`) | core | `RwLock<HashMap<url_path, EprProjectionView>>`; longest-prefix `dispatch`; atomic `replace_all` | 8 unit tests; **no boot-fetch→router integration test** | **elohim-core** |
| `dispatch_to_projected_epr` (`server/http.rs:1144`) | core | serves `/` + `/lamad` by proxying to storage `/apps/{epr_id}` | **untested; bypasses doorway's own cache** (verified — see [Key finding 3](#key-boundary-findings)) | **elohim-core** |
| `projection/warm.rs` | core | HTTP-pull warm; **`#[deprecated]`, never called** | none | **elohim-core (dead)** |
| `projection/warm_stream.rs` | core | SSE warm into MongoDB `ProjectionStore`; 5-attempt backoff; `WarmupState` | 10 unit tests; **does not refresh AppFileCache slug index** | **elohim-core** |
| `cache/app_file_cache.rs` (`AppFileCacheService`) | core | MongoDB `app_file_cache`; in-flight coalescing; slug index | 12 unit tests (all over fakes; **no MongoDB-path test**) | **elohim-core** |
| `cache/tiered.rs` (`TieredBlobCache`) | core | in-mem LRU blob/chunk cache; cleanup loop | 7 unit tests; **blob tier never written on proxy path** (TODO in `doorway/CLAUDE.md`) | **elohim-core** |
| `routes/apps.rs` (`handle_app_request`, `handle_app_capability`) | core | cache-first `/apps/{slug}/{file}`; `X-Delivery-Mode`/`X-Projection-Ready` capability probe | 16 unit tests; **full flow untested** | **elohim-core** |
| `routes/storage_proxy.rs` | core | single-target `forward_to_storage` (no fan-out); `forward_blob_to_storage` writes in-mem ContentCache | 10 tests | **elohim-core** |
| `doorway-app` (Angular) | app | operator dashboard / `/threshold` | small Vitest; no e2e | **app-private** |

### Tier 5 — EPR-app delivery seam (THE flaky seam)

The seam is a *path*, not a component, and **it is entirely informal** — its contract exists only as
string literals in seeder code and comments in `apps.rs`. This is the primary target.

```
build → zip (Jenkinsfile stageSpaBlobs / build-and-zip.sh)
     → upload (/admin/seed/blob + PATCH /db/content/{slug})
     → project-epr commitment (genesis/seeder/seed-projections.ts)
     → EprRouter (boot-fetch + 30s self-heal) → dispatch_to_projected_epr
     → storage /apps/{epr_id} → [doorway warm cache — BYPASSED on this path]
     → client (+ apps-sw service worker, scope /apps/, zip-unpack fallback)
```

| Seam element | Owner | Contract | Coverage | Class |
|---|---|---|---|---|
| `app-manifest.schema.json` (delivery role) | SDK | *should* declare bundle/epr_id/url_path/entry_file/base_href/cache/SW — **all absent today** | n/a | **SDK (the gap)** |
| `apps-sw.ts` (`app/elohim-app/src/apps-sw.ts`) + compiled `assets/apps-sw.js` | app | SW intercepts `/apps/`; zip-unpack (JSZip) fallback; `_capability` HEAD probe; LAN/WAN peer scoring | **zero tests**; **no `.ts`→`.js` build step (manual sync)** | **app-private** (should become an SDK bootstrap primitive) |
| `stageSpaBlobs` (root `Jenkinsfile:223–339`) | harness | zip + upload bytes; **re-zips per doorway** (non-deterministic hash) | scope test only | **dev-tooling / delivery harness** |
| `seed-projections.ts` | harness | hand-codes in_scope_of/url_path/epr_id/entry_file/base_href + doorway topology | `seed-projections.test.ts` (body shape only) | **dev-tooling / delivery harness** |

### Tier 6 — Reference EPR-apps (built ON the SDK)

| App (bundle slug) | Owner | Role | Coverage | Class |
|---|---|---|---|---|
| `app/elohim-app` (`elohim-host-landing`) | protocol | composable view-federation shell + landing/CMS | 208 specs; **has `app.config.spec.ts` DI smoke test (the model to copy)** | **app-private** |
| `app/lamad` (`lamad-spa`) | protocol | the LMS | 97 specs; **NO `app.config.spec.ts`** — yet this is the bundle that white-paged | **app-private** |
| `app/imagodei-portal` (`imagodei-portal`) | protocol | auth portal; bare Lit-shell, **zero SDK tokens wired**, projection seeded but **never built/staged in CI** | 1 spec | **app-private (incomplete)** |
| `genesis/landing` | protocol | vanilla static CMS landing | zero specs | **app-private** |
| `sophia` (submodule) | upstream-fork | assessment web component | own Jest suite | **SDK (rendering layer)** |

### Tier 7 — Content + seeding · **dev-tooling / delivery harness**

| Component | Owner | Contract | Coverage | Class |
|---|---|---|---|---|
| `genesis/seeder` (`seed-projections.ts`, `blob-manager.ts`, `storage-client.ts`) | harness | authors project-epr commitments + seeds content blobs; **delivery fields hand-coded** | unit (body shape); integration **skipped unless `TEST_DOORWAY_URL`** | **dev-tooling / delivery harness** |
| `deployments.json` | harness | seed-or-skip per peer (`suspended`/`genesisPeer`) | no schema test | **dev-tooling (shared data)** |
| `genesis/` content → `elohim-import` → seed-data JSON | harness | content transformation pipeline | — | **dev-tooling** |

### Tier 8 — CI/CD pipeline · **dev-tooling / delivery harness**

| Component | Owner | Contract | Coverage | Class |
|---|---|---|---|---|
| `genesis/orchestrator` (graph-walker, pipeline-registry, strategy, build-graph.groovy, dispatch) | harness | webhook → changeset → predicted pipeline set; `[build:*]` override protocol | strong Node suite (`graph-walker`, `pipeline-registry`, etc.) | **dev-tooling / delivery harness** |
| per-project `build-manifest.json` ×11 | harness | declares pipeline metadata + dependency graph | covered by walker tests | **dev-tooling** |
| root `Jenkinsfile` (App pipeline + `stageSpaBlobs`) | harness | Angular build + deploy + blob staging | `jenkinsfile-cps-scope.test.mjs` (scope only) | **dev-tooling** (hard-codes EPR delivery — see register) |

---

## Boundary leak & duplication register

The horizontal duplication sweep + per-tier agents converged on these clusters. Each is a leak from the
**four-bucket** scheme: app-private logic copy-pasted, SDK logic living app-side, or harness logic that
should be contract-driven. *Split-blocker?* and *collapse difficulty* feed deliverable 2.

| # | Cluster | Occurrences (file:line) | Canonical home | Split-blocker | Difficulty |
|---|---|---|---|---|---|
| **L1** | **Duplicate `ELOHIM_CLIENT` token** (the live white-page cause) | SDK token `elohim-service/src/client/angular-provider.ts:16`; LOCAL token `elohim-app/src/app/elohim/providers/elohim-client.provider.ts:19` (1.1 KB "Angular version mismatch workaround"); `content.service.ts` injects LOCAL; lamad aliases LOCAL→SDK at `lamad/app.config.ts:148`; **elohim-app has NO alias** → uses a separate client instance | delete the local file; `content.service.ts` injects `@elohim/service` token; collapse in `provideElohimApp` | **YES** | easy |
| **L2** | **Per-app DI bootstrap hand-wiring** (same provider set) | `elohim-app/app.config.ts:94–110` and `lamad/app.config.ts:121–175` both wire BLOB_FETCHER/GOVERNANCE/CONTENT_ATTESTATION/ECONOMIC_EVENT_FACTORY/EVENT_API/LAMAD_HOLOCHAIN_CLIENT/ELOHIM_ENV; lamad imports **16 concrete classes** from `@app/elohim`/`@app/shefa`/`@app/imagodei` at its composition root | SDK `provideElohimApp(manifest)` (Sprint 3) | **YES** | hard |
| **L3** | **`resolveDoorwayUrl` copy-paste** | verbatim at `elohim-app/app.config.ts:44–49` and `lamad/app.config.ts:71–76`; absent from `imagodei-portal`; the same-origin rule is **not in the SDK's `detectClientMode`** at all | fold into `detectClientMode` / `provideElohimApp` | **YES** | easy |
| **L4** | **`content.service.ts` ≈ `content-backend.service.ts`** | `elohim-app/.../content.service.ts` and `lamad/.../content-backend.service.ts` — **948 lines each, ~30 divergent** (import aliases + token) | collapse to one SDK/lamad service via the token pattern | **YES** | medium |
| **L5** | **`computeHash` triplicated + Jenkinsfile per-doorway zip** | `seeder/storage-client.ts:97`, `seeder/seed-sqlite.ts:289`, `seeder/blob-manager.ts:332` + shell `sha256sum` at `Jenkinsfile:286` over a per-doorway re-zip → **divergent content addresses** (`sha256-37318f6` vs `d198ee88`) | shared `computeBlobHash` + **zip-once-upload-many** | reliability-blocker | easy |
| **L6** | **`elohim-cache-core` WASM only wired in elohim-app** | asset map only in `elohim-app/angular.json:40`; **absent in lamad + imagodei-portal**; pkg dir holds **only `package.json` (WASM never built)**; import at `@elohim/service/.../content-resolver.ts:752` → `/wasm/...js` 404 (non-fatal TS fallback) | manifest-declared bundle asset dep (Sprint 4) + `wasm-pack` build step | reliability-blocker | easy/med |
| **L7** | **`apps-sw` only registered from elohim-app** | `elohim-app/src/main.ts:11`; absent in `lamad/src/main.ts` + `imagodei-portal` | SDK bootstrap, scope derived from manifest `url_path`/`base_href` | reliability-blocker | medium |
| **L8** | **`elohim` pillar imports sibling pillars + lamad models** | 40+ `@app/lamad` model imports in `elohim-app/src/app/elohim/`; 15 `@app/imagodei`/`@app/shefa`/`@app/qahal` imports; **174 ESLint pillar-boundary violations (warn-level)** ([[project_pillar_boundary_violations_backlog]]) | shared SDK models / generated types | **YES** | hard |
| **L9** | **`elohim-app/app.config.ts:25` imports `LAMAD_HOLOCHAIN_CLIENT` from `@app/lamad`** | reference-app A → reference-app B token (circular after split) | move `LAMAD_*` tokens to a shared SDK package | **YES** | medium |
| **L10** | **Legacy identity entries duplicated across DNAs** | Human/Agent/ContributorPresence/… in BOTH `elohim` DNA (`content_store_integrity/src/lib.rs:3707`, marked legacy) and `imagodei` DNA (`imagodei_integrity/src/lib.rs:889`); DoorwayRegistration in both elohim + infrastructure DNAs | Stage-G migration; collapse to one DNA | core-internal | hard |
| **L11** | **EprProjectionView dual source w/o cross-check** | Rust `elohim-views/src/projection.rs:18` + JSON `epr-projection-view.schema.json` authoritative in parallel; 13 view schemas not in `codegen-ts.mjs INTERFACE_FILES`; 15 view schemas absent from `schema_contract.rs` | add schemas to the round-trip battery | core-internal | easy |

---

## Key boundary findings

These are the structurally significant boundary truths — the ones that change how Sprints 2–5 are scoped.

1. **The app-manifest has no delivery contract.** *(Verified: top-level keys are `id, name, version,
   description, vocabulary, rendering, projections, writeThrough, signalKinds, graduation,
   constitutionalRatios, graph`.)* It is mature for **vocabulary** but says nothing about **where the
   bundle is, how to serve it, what path to project it at, or how to cache it**. Those fields
   (`epr_id/slug/url_path/entry_file/base_href/cache_policy/sw_policy/reach`) live hand-coded in
   `seed-projections.ts`. **This is the root of the cascade** and the structural reason the harness is
   thick: delivery knowledge escaped the contract into tooling. Sprint 4's job is to bring it back.

2. **Contract and implementation split across layers for HTML5-app delivery.** `html5-app-plugin` ships
   the SW *types* (SDK); the SW *implementation* is `app/elohim-app/src/apps-sw.ts` (app-private, never
   published). A third party cannot use the delivery path without copying a service worker. The SW must
   become an SDK-owned bootstrap primitive (Workstream A).

3. **The doorway's own warm cache is out of the path for the primary routes.** *(Verified at
   `server/http.rs:1144–1272`.)* `dispatch_to_projected_epr` — which serves `/` and `/lamad` — builds a
   throwaway `reqwest::Client` and proxies straight to storage. Its own comment states the intent:
   *"Storage's slug_index and AppFileCacheService handle caching; doorway proxies, not owns."* So the
   doorway's MongoDB `AppFileCacheService` (the warm projection cache the whole reliability story is
   about) only ever serves **direct `/apps/` fetches**, never the EPR-router-dispatched SPA entry points.
   The fast path is structurally bypassed on the routes that matter. This is deliverable 3's headline P1.

4. **`elohim/sdk/src/` is misclassified.** Despite living in the SDK tree and being described as
   "hand-written SDK helpers," it is a **full Holochain conductor WebSocket client** (`ElohimSDK`, admin/app
   ws). That is `elohim-core` infrastructure, not a third-party SDK surface. It must be renamed/moved
   before any public SDK release.

5. **`elohim/sdk/domains/<pillar>/` is app vocabulary inside the SDK tree.** `elohim/sdk/CLAUDE.md` itself
   says these are the "domain layer," not the "protocol layer." The *shapes* (`app-manifest.schema.json`,
   `pillar-projection.schema.json`) stay in core; the *instances* (lamad/shefa/qahal manifests) belong
   with their reference apps in the split.

6. **The "elohim DNA" is physically named `lamad`.** *(role `lamad` in `happ.yaml`; `dna.yaml:21 name:
   lamad`.)* The memory ([[project_elohim_dna_as_sdk_boundary]]) calls it the elohim DNA; there is no
   `elohim.dna`. Any third-party tooling that looks for a role named `elohim` fails silently. Reconcile in
   docs/tooling before publishing the SDK.

7. **The harness carries delivery authority it shouldn't.** `stageSpaBlobs` (in the *app* pipeline) does a
   *core seeding* operation; `seed-projections.ts` hard-codes the alpha topology rather than reading
   `deployments.json`. This is the user's framing made concrete: CI + seeder are not app-layers, they are
   the packaging/delivery harness, and their thickness is the symptom. The fix is not "tidy the harness"
   — it is "mature the manifest so the harness becomes a thin consumer."

---

## Coverage classification summary (cross-tier)

| Tier | Test kinds present | Biggest gap | Risk if unfixed |
|---|---|---|---|
| 1 — contract | schema-validation, DNA hygiene, ts-rs contract | **no bundle-delivery block to test against** | Workstream B unprovable; cascade root persists |
| 2 — SDK | TS type tests, library unit specs | **no SDK-owned bootstrap to test** (DI hand-rolled per app) | white-page class recurs per app |
| 3 — substrate | strong Rust contract + unit | spa-bundle slug_index + `extract_app_context` shadow have **no regression test** | "App not found" / empty router silently re-open |
| 4 — doorway | good path-parse + router unit tests | **`extract_app_context` shadow untested; integration tests all `#[ignore]`** | routing-shadow re-opens; router empties; CI green |
| 5 — delivery seam | a2o `spa-bundle-delivery.feature` **mostly `@wip`** | **no render test; `apps-sw` zero tests; only `text/html` + shell-marker asserted** | fallback masquerades as default; regressions ship green |
| 6 — reference apps | elohim-app **has** `app.config.spec.ts`; pillar specs | **lamad + imagodei-portal have NO DI smoke test** (the apps that white-paged) | exact white-page class has no unit gate |
| 7 — seeding | seeder unit (body shape) | integration **skipped** unless `TEST_DOORWAY_URL`; no NULL-scope regression | seed-scope drift recurs; caught only by live probing |
| 8 — CI/CD | solid orchestrator Node suite | **the post-deploy gate is itself a shell-check false-positive** (`<title>` + `app-root` visibility) | "CI green ≠ delivered" — the headline anti-pattern |

The single highest-leverage coverage gap: **there is no executable test anywhere that asserts a doorway
actually *renders* an app** (vs returning the static shell). The render-asserting machinery exists
(`ssr-hydration.steps.ts` does real `textContent` comparison; `ssr_smoke.rs` asserts `ngh=`) but is
`@wip`/`#[ignore]` and targets local fixtures, never the deployed doorways. Deliverable 3 specifies the
render-verification harness that closes this.

---

## What this map hands forward

- **→ Deliverable 2 (split decision):** the four-bucket classification + the leak register (esp. L1, L2,
  L8, L9 as split-blockers) + the harness-thinness goal.
- **→ Deliverable 3 (reliability backlog):** every "reliability finding" row, code-grounded, prioritized.
- **→ Sprint 3 (SDK + bootstrap):** L1–L4, L7, Key findings 1–4 — the SDK must own the bootstrap and
  collapse the duplicate token *at source*.
- **→ Sprint 4 (manifest):** Key finding 1 — mature `app-manifest.schema.json` with the delivery block so
  the harness (Tier 7/8) thins into a manifest-driven consumer.
