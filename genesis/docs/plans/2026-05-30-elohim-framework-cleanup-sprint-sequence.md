---
id: elohim-framework-cleanup-sprint-sequence
status: Draft
cites:
  - ../superpowers/specs/2026-05-30-elohim-sdk-epr-app-boundaries-sprint-kickoff.md   # the related doc this derives from
---

# Elohim Framework Cleanup — Sprint Sequence

Goal: turn the substrate into a **clean, reliable (our apps load), developer-friendly framework**
for building EPR-apps — so we *and* third parties can build/manifest/seed/project/serve an
EPR-app against a stable Elohim SDK + app-manifest, and it loads fast on every doorway.

Seed brief (the why + full boundary inventory + workstreams A–E):
`genesis/docs/superpowers/specs/2026-05-30-elohim-sdk-epr-app-boundaries-sprint-kickoff.md`.

Each sprint below is a self-contained, subagent-driven `/shift` (or `/deliver` for the
delivery one). Each STARTS with `superpowers:brainstorming` → spec → plan before implementing,
and `p2p-design-gate` before any data-entity/projection design. Run them in order; each builds
on the prior. Specialists noted per sprint.

## Sequence rationale (order)
`Map → Make-them-load → Clean-the-contract → Prove-conformance → Split`. You can't cleanly fix
reliability without the boundary map (last night was 7-layer whack-a-mole *because* there was no
map), so classification is first. Reliability comes next because "apps load" is the acute pain.
The SDK contract + standard bootstrap then make it durable; the app-manifest proves third-party
conformance on top of a clean SDK; the pipeline/repo split is executed last, against a clean
system, using the decision made in Sprint 1.

---

## Sprint 1 — Classify & Map (Workstream D + the E *decision*)
**Why first:** the foundation. No clean/reliable work without knowing the boundaries + contracts.
**Objective:** Produce the canonical system-boundary map: for every component (the 8 tiers in the
kickoff brief), record owner, contract/interface, test coverage, and classification —
**SDK / elohim-core / app-private**. Decide the elohim-core ↔ reference-apps split shape (data for
Sprint 5). Compile the reliability backlog (from last night + the adam/zip findings) so Sprint 2
is fully scoped.
**Scope:** read-only analysis + docs (`genesis/docs/...`, the boundary map). No code changes.
**Specialists:** `pattern-hunter` (duplication/leak detection), `code-reviewer` (contract review),
`ci-investigator` (pipeline/orchestrator boundary), `rust-architect` + `angular-architect`
(per-tier contract sanity), `quality-architect` (test-coverage classification + strategy).
**Exit / done:**
- Boundary map doc committed (every component classified SDK/core/app-private with contract + coverage).
- Split-decision doc (core vs reference-apps; what moves, what stays, versioning model).
- Reliability backlog enumerated + prioritized (feeds Sprint 2).
**Artifacts:** `genesis/docs/.../boundary-map.md`, `.../core-vs-reference-split-decision.md`.

---

## Sprint 2 — Reliability: make the apps load (Workstream C)  ← most urgent
**Why second:** acute pain. Get every app loading fast + reliably on every doorway BEFORE building
contracts on a flaky base. Verify by **render, not HTML shell** (last night's measure false-positived
on a `<title>` in the static shell).
**Objective:** the warm doorway **projection cache is the reliable fast path**; the `apps-sw`
zip-unpack is a genuine last-resort fallback only. Both reference apps render on both doorways.
**Scope / key items (the hard-won backlog):**
- **Warm projection cache** actually warms + serves (`doorway/.../projection/warm.rs`+`warm_stream.rs`
  returned `{}`); the **TieredBlobCache write-on-fetch TODO** (blob tier never written on the proxy
  path — every `/blob` round-trips). doorway/CLAUDE.md flags both.
- **`apps-sw` fallback:** find its source; make it fire ONLY on a true cache miss, not as the default.
- **EprRouter population robustness** (one-shot boot-fetch → periodic self-heal already added; verify
  + add observability) and the **storage `extract_app_context`/serving correctness** (last night's
  routing-shadow class).
- **Bundle integrity — zip-determinism bug (found 2026-05-30):** `stageSpaBlobs` re-zips the dist dir
  per doorway → different content-addresses for identical bytes (alpha `sha256-37318f6` vs apex
  `sha256-d198ee88`). Fix: **zip once, upload the single archive to all doorways.** Restores
  content-addressing integrity + cache coherence.
- **apex/adam projection leg:** adam's `lamad`+`imagodei` conductor cells are **`CellDisabled`** →
  no DHT projection → empty router → apex `/`→/threshold, `/lamad`→404. **OPERATOR PREREQ:**
  `enable_app` on adam's conductor admin API (or reset conductor state); confirm conductor-data PVC
  strategy so cells don't come back disabled on redeploy. (Code/CI can't fix this; flag + coordinate.)
  Then verify the in_scope_of-backfill projection-receive path threads through on adam.
- Define + measure **delivery SLOs:** warm-hit %, time-to-render, zero-SW-fallback; render-verified
  (a2o Playwright against the deployed doorways, or an equivalent headless render check — NOT a curl
  of the shell).
**Specialists:** `rust-architect` (doorway cache/warm/blob-tier, storage serving, EprRouter),
`ci-investigator`/`ci-observer` (deploy + Loki verification), `angular-architect` (sw + render),
`after-action` (close the last-night incident formally once green).
**Exit / done:** alpha + apex both render landing + lamad as **warm-cache fast hits, no `apps-sw`
zip-unpack**, render-verified; bundle hashes identical across doorways; SLOs met. (apex contingent
on the operator cell-enable — track that as an explicit dependency, don't block the code work on it.)

---

## Sprint 3 — Elohim SDK contract + standard app-bootstrap (Workstream A)
**Why third:** makes reliability *durable* + the framework dev-friendly. The white-page class
(missing DI providers) recurs until the SDK OWNS the bootstrap instead of each app hand-wiring it.
**Objective:** `@elohim/service` + the schemas + `storage-client-ts` + the EPR/projection contract
become a clean, documented, tested SDK surface — with an SDK-owned **standard app-bootstrap**
(`provideElohimApp(manifest)` / `bootstrapEprApp(...)`) that wires providers (client, blob-fetcher,
governance, attestation, event-factory, holochain client), same-origin doorway resolution, and
cache/SW policy **for free**. Migrate the existing reference apps onto it (proves it + deletes the
per-app hand-wiring → the Workstream-D reuse payoff).
**Scope:** `app/elohim-library/projects/*`, `elohim/sdk/*`, `app/elohim-app` + `app/lamad` +
`app/imagodei-portal` bootstraps (migrate to the standard bootstrap), `app/elohim-elements`.
**Specialists:** `angular-architect` (the bootstrap + app migration), `rust-architect` (storage-client/
schema/EPR contract + ts-rs boundary), `component-architect`+`graphos-designer` (elements/library
surface), `code-reviewer` (API surface review), `quality-architect` (contract tests).
**Exit / done:** the SDK is documented + tested + versioned; all reference apps boot via the standard
bootstrap with **zero hand-wired providers**; a "hello-EPR-app" boots on the SDK in a test harness.

---

## Sprint 4 — app-manifest + reference-app conformance (Workstream B)
**Why fourth:** the developer-friendliness proof, built ON the clean SDK (Sprint 3).
**Objective:** mature `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` into the real
contract (bundle identity/epr_id, renderer, routes/url_path, projection scope — doorway-agnostic,
reach, entry_file, base_href, cache + SW policy). **Manifest-drive** build/seed/project so there are
no hand-coded slugs/scopes/doorwayIds (kills the drift class from last night's layers 5–6). Deliver
ONE reference EPR-app that conforms end-to-end (build→manifest→seed→project→serve→render) with no
substrate hand-wiring — the third-party-readiness proof.
**Scope:** `elohim/sdk/schemas/v1/manifest*`, `genesis/seeder/*` (manifest-driven seed/project),
root `Jenkinsfile`/`stageSpaBlobs` (manifest-driven upload), the chosen reference app, docs.
**Specialists:** `rust-architect` (storage/manifest contract), `content-pipeline` (seed/project),
`ci-investigator` (pipeline integration), `angular-architect` (the reference app), `quality-architect`.
**Exit / done:** a manifest fully describes an EPR-app; the pipeline + SDK consume it (no hand-wiring);
a simulated third-party app deploys from manifest + SDK with **no substrate code changes** + renders
on both doorways. Write the "build your own EPR-app" developer guide.

---

## Sprint 5 — Pipeline / monorepo split execution (Workstream E)
**Why last:** structural; cleanest to split an already-clean, contract-driven system. Uses the
Sprint-1 decision.
**Objective:** execute the **elohim-core** (dna, storage, doorway, steward, epr, sdk, elohim-library,
bridges) ↔ **reference-apps** (Landing/CMS, Lamad/LMS, Shefa, Avodah, Mishpat, Qahal) separation, so
the core contract versions independently and reference apps build/deploy against the stable SDK.
**Scope:** `genesis/orchestrator/*` (graph-walker, dispatch), per-project `build-manifest.json`, repo
layout/workspace config, CI pipeline topology. (Read `feedback_understand_orchestrator_substrate_before_changes`.)
**Specialists:** `ci-investigator` (orchestrator/graph-walker), `rust-architect`+`angular-architect`
(workspace boundaries), `pattern-hunter` (residual cross-tier coupling).
**Exit / done:** core + reference-apps build/version/deploy independently; a reference app builds
against the published SDK (not in-tree source); orchestrator dispatch is clean + predicted (principle 7).

---

## Cross-cutting notes
- **Operator dependency (Sprint 2):** adam conductor cells `CellDisabled` — needs operator `enable_app`
  + conductor-data PVC confirmation. Surface early; don't block the code path on it.
- **Verify by render, not shell** — bake this into Sprint 2's SLO harness; it's the lesson that cost a night.
- Each sprint: brainstorm → spec (`genesis/docs/superpowers/specs/`) → plan (here) → subagent-driven
  `/shift` or `/deliver`; stability-gated done; render/contract-tested, not metric-only.
- Read first: `CLAUDE.md`, the per-crate `CLAUDE.md`s, `.claude/shifts/2026-05-30T03-15-...sprint-result.md`,
  memory `project_epr_projection_serving_chain` + `project_alpha_edge_deploy_debugging_landmarks`.

## Render-validated findings (2026-05-30 PM — live browser, post DI-fix deploy)
After the bootstrap-DI fix deployed, a real browser confirmed PROGRESS but more layers — proof
the per-app patch approach is whack-a-mole and the standard-bootstrap (Sprint 3) is the cure:
- **alpha `/lamad` partially renders** (omnibar + shell) but STILL `NullInjectorError: No provider
  for ElohimClient` via `LamadAgent → … → ElohimClient`. Root: **duplicate `ELOHIM_CLIENT`
  tokens** — `provideElohimClient` registers the `@elohim/service` token, but `ElohimAgentService`
  (`@app/elohim`, pulled into the lamad bundle) injects the **elohim-app-LOCAL** `ELOHIM_CLIENT`
  token (`elohim-client.provider.ts`, "Angular version mismatch workaround"). → Sprint 3 must
  COLLAPSE the two tokens to one (or alias), and the audit must trace the FULL transitive graph
  incl. the @app/elohim/@shefa/@imagodei services the bundle imports — not one token at a time.
- **`GET /wasm/elohim-cache-core/elohim_cache_core.js` 404** (both bundles) — the projected bundle
  expects an `elohim-cache-core` WASM at `/wasm/` that isn't built/served into the bundle. →
  Sprint 2 (asset/build-serving) + Sprint 4 (manifest must declare bundle asset deps).
- **landing white-screens via the `apps-sw` COMPRESSED fallback** — console: `deliveryMode=compressed,
  extracted ready=false`, served from the SW zip path (not warm cache); refresh → doorway 404. This
  is the user's original concern, now actively breaking landing. → Sprint 2 (warm cache as fast
  path; SW extraction readiness; refresh/SPA-fallback routing).
- **bundle hash divergence per doorway** (alpha landing `sha256-65aa8…` vs #1489's `sha256-8a9481f`;
  lamad alpha `37318f6` vs apex `d198ee88`) — the zip-determinism bug; content-addressing is
  currently meaningless. → Sprint 2 (zip-once-upload-many).

### Baseline fixes SHIPPED 2026-05-30 PM (a4ca81d89, 3e2baaa68) — so sprints start from working code
- **DI fully resolved (lamad bundle renders):** collapsed the duplicate `ELOHIM_CLIENT` (alias the
  elohim-app-local token → `@elohim/service` client) + provided `EVENT_API`, `AGENT_CONTEXT`, and the
  earlier `LAMAD_HOLOCHAIN_CLIENT`/`BLOB_FETCHER`/`GOVERNANCE`/`CONTENT_ATTESTATION`. Full transitive
  audit, both bundles. (Sprint 3 still owns the DURABLE fix: an SDK-owned standard bootstrap + collapsing
  the two tokens at the source so this can't recur; lazy-route consumers remain an audit risk until then.)
- **Landing delivery INTERIM fix:** doorway now prefers `extracted` whenever storage is reachable
  (storage serves `/apps/{slug}/{file}` on demand) → the apps-sw uses the reliable per-file path instead
  of the async compressed-zip fallback; `apps-sw` CACHE_NAME v1→v2 drops the stale index.html. Reliable,
  **not yet blazing.**
- **Still Sprint 2 (the real reliability):** warm doorway projection cache as the FAST path +
  `app_file_cache`/MongoDB wiring (it was `None` → no cache upgrade); deterministic zip hashing
  (zip-once-upload-many); automatic SW cache-invalidation on hash rotation; server-side extraction
  pre-warm after deploy; blob-tier write-on-fetch; the `/wasm/elohim-cache-core` build dependency
  (currently non-fatal via TS fallback).
- **Operator dependencies (not code/CI):** apex `/lamad`+`/` need `enable_app` for adam's `lamad`/`imagodei`
  conductor cells (`CellDisabled`) + conductor-data PVC confirmation. Until then, apex serves nothing
  through its router regardless of the above.
