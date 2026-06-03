# EPR-App Delivery Reliability Backlog (scopes Sprint 2)

> **Sprint 1 "Classify & Map" deliverable 3 of 3.** The prioritized, code-grounded defect list that
> Sprint 2 ("make the apps load") executes. Read-only analysis; no code changed.
> Companions: [`boundary-map.md`](./2026-05-30-boundary-map.md),
> [`core-vs-reference-split-decision.md`](./2026-05-30-core-vs-reference-split-decision.md).

## EPR-app serving chain (every link must hold)

Before reading the defects, know the full chain that serves `/` and `/lamad` through a doorway:

1. **Build + upload** — Angular bundles built, uploaded as content rows (`elohim-host-landing` as
   `html5-app`, `lamad-spa` as `spa-bundle`) via `stageSpaBlobs` → `PUT /admin/seed/blob` + `PATCH /db/content/{slug}`.
2. **Seeding** — project-epr REA commitments seeded (`seed-projections.ts` → `POST /api/v1/commitments`)
   with `in_scope_of = doorway:{doorwayId}|epr:{eprId}`, scoped **per doorway** (not shared).
3. **Router population** — doorway `EprRouter` populated from
   `GET {STORAGE_URL}/db/rea_commitments?action=project-epr&doorwayId={id}` — expects a **bare
   `Vec<EprProjectionView>` JSON array** (not a `{items,count}` wrapper; the wrapper breaks the decode).
   Populated at boot + 30s self-heal refresh + SSE `projection.registered`.
4. **Dispatch** — `GET /lamad` → `EprRouter` longest-prefix match → `dispatch_to_projected_epr` →
   proxy `{STORAGE_URL}/apps/{epr_id}/{sub_path}` → storage `slug_index` resolves the blob.

The R-items below are the failure modes at each link. The critical diagnostic signal: if
`/db/rea_commitments?action=project-epr&doorwayId={id}` returns `{contentCount,uniqueTags}` (the
`DbStats` shape) instead of an array, `extract_app_context` has shadowed the route to "stats" (see R14).
The `/health` endpoint does **not** touch diesel — a pod can be Ready while its DB layer is jammed; probe
the commitments route directly.

---

## Root-cause framing

Almost every item below traces to one structural fact: **the EPR-app delivery contract is hand-coded in
the dev/delivery harness instead of declared in the SDK app-manifest.** The warm-cache fast path, the
zip-determinism, the DI white-page, the SW fallback — these are not independent bugs, they are the
symptoms of an informal seam. Sprint 2 fixes the acute reliability defects so the apps load *reliably and
fast*; Sprints 3–4 make the fix *durable* by moving the contract into the SDK. **Verify every fix by
render, not by HTTP shell** — last night's measure false-positived on a static `<title>`.

**Priority key:** P0 = blocks "apps load fast & reliably" / the headline measure; P1 = the warm-cache fast
path & its correctness; P2 = hardening, dev-parity, observability. **Lane:** `code` (fixable in this
repo), `CI` (pipeline), `operator` (cluster ops — no kubectl from dev env), `test`.

---

## P0 — the apps must load fast, reliably, and *provably*

| # | Defect | Evidence (file:line) | Verified | Fix shape | Lane |
|---|---|---|---|---|---|
| **R1** | **The warm cache is bypassed on the primary routes.** `dispatch_to_projected_epr` (serves `/` and `/lamad`) builds a throwaway `reqwest::Client` and proxies straight to storage; comment: *"doorway proxies, not owns."* The doorway's MongoDB `AppFileCacheService` only serves direct `/apps/` fetches → SPA entry points never hit the warm cache. | `doorway/.../server/http.rs:1144–1272` (esp. 1217–1231) | **yes (synthesizer-verified)** | extract a shared `serve_app_file(state, epr_id, sub_path)` from `routes/apps.rs:handle_app_request` and call it from `dispatch_to_projected_epr` so both paths are cache-first | code |
| **R2** | **Zip-determinism: same bytes → different content-address per doorway.** `stageSpaBlobs` re-zips the dist dir inside a per-doorway loop (`zip -r`, no `-X`); `cp index.csr.html index.html` perturbs mtime → alpha `sha256-37318f6` vs apex `sha256-d198ee88`. Content-addressing is meaningless; cross-doorway cache never coheres; forces SW zip fallback. | `Jenkinsfile:265–339` (zip at :285, loop at :1108–1113); plan §"Render-validated findings" | **yes** | **zip once, sha256 once, upload the single archive to all doorways**; add reproducible flags; dedupe the 3 TS `computeHash` copies (L5) | CI |
| **R3** | **`apps-sw` zip-unpack fires as the *default*, not a last resort.** SW intercepts all `/apps/` unconditionally; when `X-Projection-Ready=false` it runs `fetchViaZip` (JSZip) → white-screen during async extraction; refresh → doorway 404. The `false` is *caused by* R1 (cache bypass → `resolve_blob_hash`=None). | `app/elohim-app/src/apps-sw.ts:149–245`; `main.ts:11` | **yes** | R1 transitively flips `X-Projection-Ready` true; additionally make the SW fire zip **only** on a true cache miss; fix SPA-fallback so refresh on a sub-path serves the entry file, not 404 | code |
| **R4** | **No render test anywhere; the post-deploy gate is a shell false-positive.** The CI gate asserts the static `<title>` + `app-root` *visibility* — both present on a white page. Render-asserting tests (`ssr-hydration.steps.ts`, `ssr_smoke.rs`) exist but are `@wip`/`#[ignore]` against local fixtures. | `genesis/a2o/steps/ui/navigation.steps.ts:107–110,146–155`; `Jenkinsfile:148–166` | **yes** | ship the **render-verification SLO harness** (below): Playwright against *both* deployed doorways asserting real `textContent`, not the shell | test/CI |
| **R5** | **lamad + imagodei-portal have no DI smoke test** — the exact bundles that white-paged. elohim-app *has* `app.config.spec.ts` (the model). | `app/lamad/src/app/app.config.ts` (no sibling spec); `app/elohim-app/src/app/app.config.spec.ts:35–76` | **yes** | port `app.config.spec.ts` to lamad + imagodei-portal: `TestBed` boot the **production** provider graph, assert every cross-pillar token resolves | test |

---

## P1 — warm-cache fast path & projection correctness

| # | Defect | Evidence (file:line) | Verified | Fix shape | Lane |
|---|---|---|---|---|---|
| **R6** | **Projection signal subscribers bound to the wrong cell.** `subscribe_rea_projection_signals` + `subscribe_elohim_content_signals` are inside `if let Some(hc) = registry.infrastructure` but project-epr signals fire from the **lamad** cell → SSE repopulation never fires; the router only heals via the 30s HTTP poll. ("Theory B — NOT fixed.") | `elohim/elohim-storage/src/main.rs:596–757`; `project_alpha_edge_deploy_debugging_landmarks:72–78` | **yes (code-read)** | bind the subscribers to the lamad/`content_store` client, not `infrastructure` | code |
| **R7** | **`warm_stream` doesn't refresh the AppFileCache slug index.** It writes content into MongoDB `projected_entries` but nothing calls `load_slug_index()` afterward → first `/apps/{slug}/index.html` on a cold pod returns `resolve_blob_hash=None` → `X-Cache: BYPASS` until a content event arrives. | `doorway/.../projection/warm_stream.rs:269`; `cache/app_file_cache.rs` | **yes** | on stream completion, `refresh_app(slug)` for each html5-app/spa-bundle entry (or `load_slug_index()` once) | code |
| **R8** | **`TieredBlobCache` blob tier never written on the proxy path** → every `/blob/<hash>` round-trips to storage. (Documented TODO.) | `doorway/CLAUDE.md:51–53`; `routes/storage_proxy.rs` (`forward_to_storage`); `set_blob` has zero non-test callers | **yes** | populate the tiered cache after a successful blob 200 (`set_blob(hash, body, reach)`) | code |
| **R9** | **EprRouter boot race.** Boot-fetch (10s timeout) races storage DB readiness; on miss the router is empty for up to `DOORWAY_EPR_REFRESH_SECS` (30s) → `/`→/threshold, `/lamad`→404 every restart. | `doorway/.../main.rs:562–677`; landmarks memory | **yes** | probe `/db/rea_commitments?action=project-epr` (not `/health` — it skips diesel) before serving; tighten refresh while the router is empty; add a populated-router metric | code |
| **R10** | **The apex/adam projection leg — still open.** adam returns `[]` for `apex-elohim-host` project-epr rows while matthew (alpha) has them repaired. **Two coupled causes:** (a) **operator** — adam's `lamad`+`imagodei` conductor cells are `CellDisabled`; (b) **architecture/seeder** — deeper code-read indicates Holochain `post_commit` fires only on the *local* agent's commits, so gossiped commitments never trigger adam's projection; the row lands on adam only if **the seeder POSTs project-epr to adam's own storage** (each backend authors locally), not via DHT gossip. | sprint-result:50–60; `elohim/elohim-storage/src/main.rs:680–707`, `rea_projection.rs:337–368` | **partial** (needs confirmation) | (a) operator `enable_app` on adam + confirm conductor-data PVC survives redeploy; (b) confirm seeder posts to both storage backends; if relying on gossip, that path doesn't exist — make seeding per-backend explicit | operator + code |
| **R11** | **`/wasm/elohim-cache-core/elohim_cache_core.js` 404 on the projected bundles.** The pkg dir holds only `package.json` (WASM never built); the asset is mapped only in elohim-app's `angular.json`, absent in lamad + imagodei-portal; import at `@elohim/service/.../content-resolver.ts:752`. Non-fatal (TS fallback) but noisy and misleads observability. | `elohim/elohim-cache-core/pkg/` (1 file); `app/lamad/angular.json` (no wasm glob); `content-resolver.ts:752` | **yes** | add `wasm-pack build --target web` to the app prebuild **and** declare the bundle asset dep in the manifest (Sprint 4); or promote the TS fallback to primary and drop the WASM path until ready | CI/code |
| **R12** | **`apps-sw.ts` → `assets/apps-sw.js` has no build step** — the compiled SW is hand-committed; source/asset can silently diverge (e.g. `CACHE_NAME v1→v2`). | `app/elohim-app/src/main.ts:12` (registers `/apps-sw.js`); `src/apps-sw.ts` vs `src/assets/apps-sw.js` | **yes** | add `esbuild src/apps-sw.ts → assets/apps-sw.js` to the Angular build; remove the manual-sync risk | CI |
| **R13** | **`app_file_cache=None` on the non-Mongo boot paths** → dev (and any no-Mongo) deploy always `X-Cache: BYPASS`; symptom `appFileCache.cachedFiles:0`. | `doorway/.../server/http.rs:379,471,578` (None) vs `:638,729` (Some) | **yes** | ensure MongoDB is wired in the deployed config; loud-warn when `app_file_cache` is None in a prod profile | operator/code |

---

## P2 — hardening, regression gates, observability

| # | Defect | Evidence | Fix shape | Lane |
|---|---|---|---|---|
| **R14** | **`extract_app_context` routing-shadow has no regression test** — a new single-segment `/db/{x}` route absent from `legacy_prefixes` silently re-shadows to `handle_db_stats` (the layer-6 cascade). | `elohim/.../http.rs:2800–2829`; tests bypass it | unit test: assert `extract_app_context("rea_commitments") == "rea_commitments"` + every `handle_db_request` arm is in `legacy_prefixes` | test |
| **R15** | **spa-bundle slug_index serving has no regression test** — the layer-1/2 cascade (format filter + index-by-id). | `elohim/.../http.rs:540–559,4605` | integration test: seed a `spa-bundle` row, assert it resolves via `load_slug_index`/`lookup_slug_blob_hash` | test |
| **R16** | **`upsert_with_anchor` in_scope_of backfill untested** — the layer-7 NULL-scope repair. | `elohim/.../db/rea_commitments.rs:274,299` | DB test: insert NULL-scope row, upsert with scope, assert column updated | test |
| **R17** | **`/db/rea_commitments` bare-array contract untested** — doorway decodes `Vec<EprProjectionView>`; an `{items,count}` wrapper would silently break the router. | `project_epr_projection_serving_chain` gotcha | storage response-shape test + doorway decode test | test |
| **R18** | **`/health` doesn't touch diesel** — a pod is Ready while its DB is jammed. | `elohim/.../http.rs:1341` | add `/health?deep=true` that runs `SELECT 1`; use it as the k8s readiness probe | code/operator |
| **R19** | **15 view schemas missing from `schema_contract.rs`; 13 not distributed by `codegen-ts.mjs`** — Rust↔TS drift can pass CI (esp. `doorway-operator-binding-view`, `wisdom-invocation-response`). | deliverable 1, L11 | add the round-trip fixtures + `INTERFACE_FILES` entries | test |
| **R20** | **Per-request `reqwest::Client` on every proxy path** — no connection pooling (`dispatch_to_projected_epr:1228`, `forward_to_storage`, `apps.rs:fetch_and_cache`). | deliverable 1 | a shared `reqwest::Client` on `AppState` (`ssr_http_client` already exists — reuse it) | code |
| **R21** | **imagodei-portal: projection seeded but bundle never built/staged** — `/auth/portal` is a dangling EprRouter entry. | `seed-projections.ts:201`; `Jenkinsfile:1109–1112` (only landing + lamad-spa) | either add the build+stage step or remove the projection until the app is real | CI |
| **R22** | **seeder integration test skipped in CI** (`TEST_DOORWAY_URL`-gated); no NULL-scope/apex-scope assertion. | `genesis/seeder/.../projections-substrate.test.ts` | run it post-deploy against the stack; assert non-NULL `in_scope_of` + apex scope on both doorways | test/CI |

---

## Operator dependencies (called out — not code/CI fixable)

These cannot be closed from the dev env (no kubectl). Surface early; **do not block the code path on
them** — track as explicit dependencies (per the plan's cross-cutting note).

- **O1 — adam conductor cells `CellDisabled`** (R10a): operator `enable_app` on adam's conductor admin
  API (or reset conductor state). Without this, *no* code fix makes apex serve through its router.
- **O2 — conductor-data PVC strategy** (R10a): confirm cells don't return disabled on redeploy. A durable
  PVC or an enable-on-boot step.
- **O3 — `DOORWAY_ID` on doorway pods**: an unset `DOORWAY_ID` silently yields an empty router (it filters
  `in_scope_of LIKE '%doorway:{id}|%'`). Verify on every doorway pod (`apex-elohim-host` per `alpha-b.yaml`).
- **O4 — verify adam runs the backfill binary** (`ef5e1e2d5`) via Loki before concluding R10b is a code gap.

---

## Delivery SLOs + render-verification harness (the R4 deliverable)

The harness that closes "CI-green ≠ human-visible delivery." It runs **post-deploy against both deployed
doorways** (`alpha.elohim.host` AND `elohim.host`) — which needs no local cluster, so the `@wip` "needs a
cluster" caveat does not apply.

**SLOs (per doorway × per EPR-app):**
- **Warm-hit ≥ 99%** of `/apps/{epr}/*` asset responses served from the doorway warm cache (cache-tier
  header HIT). *Note:* depends on R1 + R8 landing the cache-tier marker on the bundle path first.
- **Zero SW zip-unpack** on a warm load — assert no `*.zip` fetched/decompressed (via the SW's own
  evidence log + `performance.getEntriesByType('resource')`).
- **Time-to-render ≤ 1500ms p95** for first meaningful content (landing hero text; a seeded lamad concept
  title), cold browser cache, warm doorway.
- **Render-NOT-shell:** success requires an app-specific, content-bearing DOM string present *after
  hydration* — explicitly **not** the static `<title>` and **not** mere `app-root` visibility.

**Harness design:**
1. Promote the `ssr-hydration.steps.ts` render pattern (`document.body.cloneNode` → strip script/style →
   compare real `textContent`) into a new **un-`@wip`** `steps/ui/delivery-render.steps.ts` +
   `features/delivery/warm-render.feature`, tagged `@e2e @browser-only` (so `genesis/Jenkinsfile:1618`
   actually runs it).
2. Run under the existing `delivery-browser` Playwright profile against `alpha` **and** a new `apex`
   target. Replace/augment the shell-only Cypress `staging-validation.feature` in `Jenkinsfile:166`.
3. Each scenario asserts the four SLOs (render-not-shell via `expect(getByText(...)).toBeVisible()` +
   `app-root` non-empty; zero-zip via a response collector; warm-hit % via headers; TTR via LCP).
4. Keep a **cheap HTTP guard in front**: assert `/db/rea_commitments?action=project-epr&doorwayId={id}`
   returns a non-empty `Vec<EprProjectionView>` for each `(doorway, epr)` — catches the empty-router class
   (cascade layers 3–7) on **both** doorways without a browser. (This is the sprint-result's own manual
   probe, made executable — and it would have caught the apex blind spot from day one.)

---

## Sprint 2 ordering (recommended)

1. **R1 + R7 + R8** (the warm-cache fast path is actually in the path and actually warms) — the core of
   "apps load *fast*."
2. **R2** (zip-once-upload-many) + **R3/R12** (SW fires only on true miss; SW has a build step) — content-
   addressing coheres; the fallback stops masquerading as default.
3. **R4 + R5 + the HTTP guard** (render-verification + DI smoke tests) — so "done" is *provable by render*.
4. **R6 + R9** (projection signal cell binding; boot race) — the router heals fast and via SSE.
5. **R10** (apex leg) — gated on **O1/O2**; pursue the code/seeder half (R10b) in parallel.
6. **R11/R13/R21** + the P2 regression gates (R14–R22) — harden so the cascade can't silently re-open.

**Exit (per plan):** alpha + apex both render landing + lamad as **warm-cache fast hits, no apps-sw
zip-unpack, verified by render**, bundle hashes identical across doorways, SLOs met. (apex contingent on
the operator cell-enable — tracked as an explicit dependency, not a code blocker.)
