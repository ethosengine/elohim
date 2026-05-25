# Cross-Pillar Import Cleanup — Lamad Bundle Independence + Elohim SDK Surfacing

> **For agentic workers:** SUBAGENT-DRIVEN parallel plan. Wave 1 sequential (blocking). Wave 2 parallel (5–7 agents). Wave 3 sequential (cutover). Use **superpowers:subagent-driven-development** + **superpowers:dispatching-parallel-agents** to execute.

**Goal:** Migrate **159 cross-pillar imports** out of `app/lamad/` source so the lamad bundle builds STANDALONE without reaching into elohim-app source. Two compounding deliverables: (a) **surface and clarify the Elohim-core SDK boundary** (what belongs in `@elohim/service` / `elohim-core` / `@elohim/storage-client` vs. what stays in pillar source); (b) **capture the recurring patterns as elohim-native dev tooling documentation** so the next pillar split (shefa, qahal, …) follows a runbook instead of re-deriving from scratch.

**Architecture:** Three waves, fan-in-fan-out shape.

- **Wave 1** (1 agent, sequential, blocking ~30 min) — Disposition manifest. Classify each of 159 imports into one of 8 dispositions. Produces the input contract for Wave 2.
- **Wave 2** (7 agents, parallel, ~2–4 hours each) — Independent migration slices. File-disjoint scope. Each agent works one disposition slice.
- **Wave 3** (1 agent, sequential after all Wave 2, ~2 hours) — Cutover: remove cross-pillar aliases, verify bundle independence, document the new SDK boundary as canon, commit the milestone.

**Tech stack:** TypeScript, Angular 19, pnpm workspaces, JSON Schema codegen, vitest, Lit (custom elements).

---

## P2P Design Gate output (no new entities)

This sprint is **import reorganization only**. It does not introduce new DHT entry types, new SQLite tables, new wire-format schemas, new EconomicEvent kinds, new Commitment actions, new FeedbackSignal kinds, or new substrate primitives of any kind. Every symbol migrated already exists; only its filesystem location and import path change.

Existing entities the migrated symbols touch (all source-of-truth already declared elsewhere):

| Entity touched by migrated symbols | Classification | Source of truth | Authoritative spec |
| --- | --- | --- | --- |
| Content (rows + blobs) | Notarized (A) | Holochain DHT (`elohim` zome `ContentEntry`); projected to `content` SQLite table | `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` |
| Project-EPR Commitment | Notarized (A) | Holochain DHT (Mishpat `Commitment` with `project-epr` action) | Same as above |
| EPR envelope | Notarized (A) | Holochain DHT via `FederatedEprStore` + Kad `StartProviding` | `elohim/elohim-storage/src/api/epr.rs:484` |
| Agent / identity | Notarized (A) | Holochain DHT (`imagodei` zome `Agent` entry) | `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` |
| EconomicEvent (existing actions) | Notarized (A) | Holochain DHT (elohim zome) | Existing manifest |
| FeedbackSignal (existing kinds) | Derived (A2) | Holochain DHT (elohim zome `FeedbackSignal` + signal_kind extension) | `project_signal_kind_extensible_protocol_class` memory |

`@elohim/service`, `elohim-core`, `@elohim/identity`, `@elohim/rea-runtime`, `@elohim/storage-client` are **TypeScript library packages** that hold operational client code consuming the above substrate entities. They are Category C (operational) under the P2P Design Gate taxonomy — they hold no source-of-truth state themselves; everything they expose is reconstructible from substrate.

Audit lines flagging "new storage schema" against `@elohim/*` package mentions in this plan are pattern-match false positives — those tokens reference TypeScript package names, not JSON Schema files.

---

## Pre-sprint readiness checklist

Before dispatching Wave 1, the operator verifies:

- [ ] **Substrate baseline landed.** Z.D Phase 1 schemas (REA compute-commitment + delegates-compute) are on dev (commits `b2380b899`, `7f66391b6`, `bf2efd191`). The new SDK can absorb the REA primitive from day one — confirmed.
- [ ] **Lamad standalone is currently healthy.** `pnpm --filter lamad build` and `pnpm --filter lamad test` both green. As of integration of `design/peer-oauth-portal` (commits `a224c3e79..e18c4cb48`), lamad shows: build clean (warnings only — sass deprecation + NG8107), 2775 tests across 98 files all green, 126 cross-pillar import lines (228 `@app/elohim`, 28 `@app/imagodei`, 11 `@app/shefa`, 9 `@app/qahal`).
- [ ] **Pre-existing test debt audited.** The peer-OAuth-portal integration surfaced two test files that were obsolete after landed refactors (epr-link spec post-Lit-wrapper refactor `10516614e`; app.routes spec post-lamad-bundle-split `effc26e04`). Before each Wave 2 slice dispatches, the slice agent runs `grep -rln "ngOnChanges\|loadChildren.*lamad" app/<consumer-bundle>/src --include="*.spec.ts"` and similar audits for the symbols it's migrating — obsolete specs surface BEFORE they block the pre-push gate.
- [ ] **Pnpm lockfile is clean.** `pnpm install --lockfile-only` produces no diff. (When a Wave 2 slice scaffolds a new Angular library, it MUST re-run lockfile reconciliation; the workspace add was caught during peer-OAuth-portal integration and is captured in Slice 2.7's runbook.)
- [ ] **Doorway clippy is healthy.** `cargo clippy -- -D warnings` on `doorway/doorway-service` returns 0. A regression at `src/server/http.rs:1257` (commit `542ca8f0b`) blocked the elohim-edge Docker image build for build 1002; fixed in `8c66fa3ca` before this sprint can land any HTTP-route changes from Slice 2.6.

---

## Context

The pillar-EPR decomposition merge (dev commit `0b36155e9`) moved lamad into `app/lamad/` as its own SPA bundle. But B18b configured lamad's `tsconfig.json` with cross-pillar path aliases pointing AT `app/elohim-app/src/app/...`:

```jsonc
{
  "paths": {
    "@app/elohim/*":   ["../elohim-app/src/app/elohim/*"],
    "@app/imagodei/*": ["../elohim-app/src/app/imagodei/*"],
    "@app/qahal/*":    ["../elohim-app/src/app/qahal/*"],
    "@app/shefa/*":    ["../elohim-app/src/app/shefa/*"],
    "@app/generated/*":["../elohim-app/src/app/generated/*"]
  }
}
```

That means **lamad reaches across the bundle boundary into elohim-app's source tree** at compile time. The build works; the dev mode works. But it violates the bundle independence principle pillar-EPR is supposed to enable. Each pillar bundle should resolve its dependencies through shared libraries (`@elohim/service`, `elohim-core`, `@elohim/storage-client`) or substrate APIs (doorway HTTP, EPR resolution), not by source-reach.

The B0 audit (`genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md`) identified the 159 cross-pillar imports — broken down as:

| Pillar | Imports | Unique paths |
| --- | --- | --- |
| `@app/elohim/*` | 127 | 46 |
| `@app/imagodei/*` | 11 | 7 |
| `@app/qahal/*` | 5 | 4 |
| `@app/shefa/*` | 5 | 5 |
| `@app/generated/*` | 3 | 2 |

This plan retires that transitional debt and produces a clean SDK boundary that future pillar splits can rely on.

---

## Eight Disposition Codes

Each import in the manifest gets classified into one of these:

| Code | Disposition | Examples | Target location |
| --- | --- | --- | --- |
| **L** | Move to `@elohim/service` library | `ContentService`, `EprResolverService`, `DoorwayClientService`, `IndexeddbCacheService`, `ProjectionApiService` | `app/elohim-library/projects/elohim-service/src/` |
| **C** | Move to `elohim-core` Lit element library | Cross-pillar UI primitives (`reaction-bar`, `graduated-feedback`) | `app/elohim-elements/elohim-core/src/` |
| **S** | Move to `@elohim/storage-client` | Wire-format types, storage RPC helpers | `elohim/sdk/storage-client-ts/src/` |
| **I** | Move to new `@elohim/identity` library | Identity guards, session services, profile model (currently `@app/imagodei/*`) | `app/elohim-library/projects/elohim-identity/src/` (new) |
| **R** | Move to new `@elohim/rea-runtime` library | REA event services, observability (currently `@app/shefa/*`) | `app/elohim-library/projects/elohim-rea-runtime/src/` (new) |
| **H** | Consume via doorway HTTP API | Cross-pillar data fetches that should ride the substrate | `doorway/doorway-service/src/routes/` |
| **E** | Consume via EPR resolution | Cross-pillar content references via `elohim-epr-link` | `<elohim-epr-link>` Lit element |
| **D** | Duplicate intentionally | Distinct copies the pillars need to evolve separately (rare, justify per case) | In-pillar |
| **X** | Delete | Unused after deeper audit | — |

Wave 1's output is the manifest mapping each of 159 imports to one of these codes plus a target file location.

---

## Wave 1 — Disposition Manifest

**Agent:** `cartographer` (Opus tier — needs vision/judgment for SDK boundary calls)
**Scope:** Read all 159 imports, classify each, produce manifest.
**Duration:** ~30–45 min.
**Output:** `genesis/docs/superpowers/notes/2026-05-25-cross-pillar-cleanup-dispositions.md`

### Steps

- [ ] **Step 1: Re-read the B0 audit**
  Re-read `genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md` to understand existing classifications.

- [ ] **Step 2: Enumerate every cross-pillar import**

  ```bash
  grep -rE "from '@app/(elohim|imagodei|qahal|shefa|doorway|avodah|generated)" app/lamad/src --include="*.ts" \
    | sort -u > /tmp/lamad-imports.txt
  ```

  Cross-check against the audit's count of 159; surface any discrepancy.

- [ ] **Step 3: Classify each import**

  Per import, decide L / C / S / I / R / H / E / D / X using these rules:

  | Rule | Disposition |
  | --- | --- |
  | Imported by 3+ pillars, no UI surface | L (library) |
  | Lit custom element with capability profile | C (elohim-core) |
  | Wire-format type / RPC | S (storage-client) |
  | Auth / identity primitive | I (new identity lib) |
  | REA Event / Commitment runtime | R (new rea-runtime lib) |
  | Cross-pillar data fetch that should ride substrate | H (doorway HTTP) |
  | Cross-pillar content reference | E (EPR link) |
  | Genuine intentional copy | D (justify) |
  | Unused | X |

  When unsure, default to **L** and flag for Wave 2 agent to confirm during migration.

- [ ] **Step 4: Write the manifest**

  Format as a markdown table — one row per import — with columns:

  | Source path | Symbol(s) | Count | Disposition | Target location | Notes |
  | --- | --- | --- | --- | --- | --- |
  | `@app/elohim/services/content.service` | `ContentService` | 23 | L | `@elohim/service/lib/content-service` | Most-imported service; first migration candidate |

  Group rows by disposition code so Wave 2 agents see their slice immediately.

- [ ] **Step 5: Surface library structure questions for the operator**

  Flag at the top of the manifest any decisions that need operator input:
  - Should `@elohim/identity` be a new library or fold into `@elohim/service`?
  - Should `@elohim/rea-runtime` be a new library or fold into `@elohim/service`?
  - For any **D** (duplicate) classification, justify per case.

- [ ] **Step 6: Commit**

  ```bash
  git add genesis/docs/superpowers/notes/2026-05-25-cross-pillar-cleanup-dispositions.md
  git commit -m "docs(plan): cross-pillar import cleanup — Wave 1 disposition manifest"
  ```

### Wave 1 acceptance signals

- Every one of the 159 imports has a row in the manifest.
- Counts in the manifest's per-disposition summary equal the audit's totals.
- All "operator input needed" decisions are surfaced for resolution before Wave 2 dispatches.

---

## Wave 2 — Parallel Migration Slices

After Wave 1's manifest lands AND operator resolves library-structure questions, dispatch 7 agents in parallel. **Each agent works a different disposition code; file scopes are disjoint.**

### Slice 2.1 — L (Move to `@elohim/service` library)

**Agent:** `angular-architect`
**Scope:** All imports classified L (~80–120 of 127 from `@app/elohim/*`)
**Files touched:**
- Add: `app/elohim-library/projects/elohim-service/src/lib/<service>.ts` (per migrated service)
- Update: `app/elohim-library/projects/elohim-service/src/public-api.ts` (re-exports)
- Update: `app/lamad/src/app/**/*.ts` (import sites — change `@app/elohim/...` → `@elohim/service`)
- Update: `app/elohim-app/src/app/**/*.ts` (same — bidirectional consumers)
- Delete: original `app/elohim-app/src/app/elohim/services/<service>.ts` (after consumers migrate)

**Steps:**

1. For each service in the L slice:
   - [ ] Identify the canonical implementation (usually the one in `app/elohim-app/src/app/elohim/services/`)
   - [ ] Copy it AND its `*.spec.ts` to `app/elohim-library/projects/elohim-service/src/lib/` (test-spec co-migration — surfaced as a gotcha during peer-OAuth-portal integration where two specs were orphaned by their components' refactors and only the pre-push gate caught them)
   - [ ] Adapt imports: any `@app/*` cross-references inside the service body need their own L migration recursively
   - [ ] Add re-export to `public-api.ts`
   - [ ] Find all consumers via grep: `grep -rln "@app/elohim/services/<name>" app/`
   - [ ] Rewrite imports in each consumer
   - [ ] Verify build: `pnpm --filter elohim-library typecheck && pnpm --filter lamad build && pnpm --filter elohim-app build`
   - [ ] Delete original source location AND the original `*.spec.ts` (no duplicate spec; the library copy is authoritative)
   - [ ] Commit per service: `feat(elohim-service): migrate <Service> from elohim-app pillar (Slice 2.1)`

2. After all L services migrate:
   - [ ] Verify `@elohim/service`'s `public-api.ts` exports every symbol lamad imports
   - [ ] Run `pnpm --filter elohim-library test` — full library test suite
   - [ ] Commit the Slice 2.1 milestone

**Acceptance signal:** Zero `@app/elohim/*` imports remain in app/lamad/ or app/elohim-app/ source.

---

### Slice 2.2 — C (UI primitives → `elohim-core` Lit library)

**Agent:** `component-architect`
**Scope:** All imports classified C (likely `@app/qahal/components/graduated-feedback`, `reaction-bar`, and any other cross-pillar UI components in lamad's deps)
**Files touched:**
- Add: `app/elohim-elements/elohim-core/src/elohim-<component>.ts` (Lit element rewrite)
- Add: corresponding `.spec.ts` (web-test-runner)
- Update: `app/elohim-elements/elohim-core/src/register.ts` (custom element registration)
- Library A default stories: `app/elohim-library/projects/graphos/stories/elohim-<component>/`
- Library B designed stories: `app/elohim-library/projects/graphos/stories/elohim-<component>/`
- Update consumers in `app/lamad/` and `app/elohim-app/` to use `<elohim-<name>>` Lit element instead of Angular component
- Delete: original Angular `*.component.{ts,html,css,spec.ts}` in `@app/qahal/components/`

**Steps:**

1. For each C component:
   - [ ] Read the Angular component to understand props, events, behavior
   - [ ] Identify the [Capability Profile](../specs/2026-05-20-capability-profile-element-contract-design.md) contract (which lenses it supports)
   - [ ] Author the Lit element with capability profile JSDoc + the three precondition gates (a11y, i18n, ua-prefs)
   - [ ] Author Library A default stories (Unstyled + CustomTheme + every claimed lens)
   - [ ] Author Library B designed stories (Elohim brand binding)
   - [ ] Author spec covering: prop binding, event dispatch, accessibility tree, customization via CSS custom properties
   - [ ] Update consumers in lamad / elohim-app to use `<elohim-<name>>` (with `CUSTOM_ELEMENTS_SCHEMA` + thin Angular wrapper if needed, like EprLinkComponent pattern)
   - [ ] Delete original Angular component
   - [ ] Commit per component

2. After all C components migrate:
   - [ ] Verify storybook builds: `pnpm --filter graphos build`
   - [ ] Commit the Slice 2.2 milestone

**Acceptance signal:** Zero `@app/qahal/components/*` (or other cross-pillar UI primitives) imports remain in lamad.

---

### Slice 2.3 — I (Identity → new `@elohim/identity` library)

**Agent:** `angular-architect`
**Scope:** All imports classified I (~11 imports from `@app/imagodei/*`)
**Files touched:**
- Create: `app/elohim-library/projects/elohim-identity/` (new library — Angular library scaffold)
- Add: identity guards, session services, profile model in `projects/elohim-identity/src/lib/`
- Update workspace `angular.json`, `tsconfig.json` to register the new library
- Update lamad + elohim-app consumers
- Delete originals in `app/elohim-app/src/app/imagodei/`

**Steps:**

1. **Operator decision check** (Wave 1 should have resolved this): is `@elohim/identity` a new library or do we fold identity into existing `@elohim/service`?
   - **Default:** new library, because identity is a load-bearing distinct concern with its own SDK surface (per `cradle-to-grave-capability-gradient.md` §4 elohim mediation roles)
2. - [ ] Scaffold `projects/elohim-identity/` (use `ng generate library` or copy from elohim-service skeleton)
3. - [ ] For each imagodei service/guard/model:
   - Migrate to `projects/elohim-identity/src/lib/`
   - Update `public-api.ts`
   - Update consumers
   - Verify build
   - Delete original
4. - [ ] Commit per service + slice milestone

**Acceptance signal:** Zero `@app/imagodei/*` imports remain in app/lamad/ or app/elohim-app/ (consumers migrated; library hosts the symbols).

---

### Slice 2.4 — R (REA → new `@elohim/rea-runtime` library OR fold into `@elohim/service`)

**Agent:** `rust-architect` (REA primitive familiarity)
**Scope:** All imports classified R (~5 imports from `@app/shefa/*` — event service, attention tracker)
**Files touched:** Same pattern as Slice 2.3 but for shefa primitives.

**Steps:**

1. **Operator decision check:** new `@elohim/rea-runtime` library or fold into `@elohim/service`?
   - **Default:** new library — REA event/commitment runtime is a load-bearing distinct primitive per [rea-compute-commitment-primitive](../../architecture/rea-compute-commitment-primitive.md)
2. - [ ] Scaffold `projects/elohim-rea-runtime/` (Angular library)
3. - [ ] Migrate event-service, attention-tracker, etc. (per Wave 1 manifest)
4. - [ ] Update consumers
5. - [ ] Delete originals

**Acceptance signal:** Zero `@app/shefa/*` imports remain in lamad.

---

### Slice 2.5 — S + Generated types distribution

**Agent:** `code-reviewer` (knows schema codegen flow)
**Scope:** All imports classified S (~3 imports from `@app/generated/*` + any storage-client primitives)
**Files touched:**
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` (if more distribution targets needed)
- `elohim/sdk/storage-client-ts/src/` (if migrations land here)
- `app/lamad/src/app/**/*.ts` (import sites)

**Steps:**

1. - [ ] Re-verify `app/lamad/src/generated/` distribution target is current (added by `48ad0f548`)
2. - [ ] For any S imports moving to `@elohim/storage-client`:
   - Add to storage-client's public-api
   - Update consumers
3. - [ ] Run `pnpm run schema:codegen:ts --verify` — confirms all generated artifacts are fresh
4. - [ ] Commit slice milestone

**Acceptance signal:** All schema-generated types resolve from `@elohim/storage-client/*` or the bundle-local `generated/` directory. No `@app/generated/*` aliases needed in lamad's tsconfig.

---

### Slice 2.6 — H + E (Substrate-API consumption)

**Agent:** `claude` (general — needs both Angular + doorway knowledge)
**Scope:** All imports classified H (consume via doorway HTTP) and E (consume via EPR resolution)
**Files touched:**
- `app/lamad/src/app/**/*.ts` (rewrite imports as HTTP calls or EPR resolutions)
- Possibly new doorway routes in `doorway/doorway-service/src/routes/`

**Steps:**

1. For each H import:
   - [ ] Identify the doorway HTTP route that serves equivalent data (or surface to add one)
   - [ ] Replace direct service call with `EprResolverService.resolve()` + projection adapter
   - [ ] Update tests
   - [ ] Commit per call site
2. For each E import:
   - [ ] Replace with `<elohim-epr-link>` Lit element + capability profile lens
   - [ ] Update tests

**Acceptance signal:** No remaining direct cross-pillar service injections; data flows through substrate.

---

### Slice 2.7 — Elohim-native dev tooling runbook + SDK boundary documentation

**Agent:** `storyteller` (canon-tier prose) + `cartographer` (synthesis from session patterns)
**Scope:** Documentation-only deliverables. Runs IN PARALLEL with Slices 2.1–2.6 (no code conflicts).
**Files touched:**
- Create: `genesis/docs/architecture/elohim-sdk.md` (canon-tier)
- Create: `genesis/docs/architecture/pillar-bundle-split-runbook.md` (operational canon)
- Update: `genesis/docs/architecture/README.md` (index)

**Steps:**

1. - [ ] Write `elohim-sdk.md` (canon):
   - What's in `@elohim/service` vs `elohim-core` vs `@elohim/storage-client` vs `@elohim/identity` vs `@elohim/rea-runtime`
   - The principle for when something belongs in each (data/transport/UI/identity/REA)
   - The substrate-API consumption patterns (doorway HTTP, EPR resolution)
   - Cradle-to-grave inheritance: how each SDK piece serves life-stage capacity transitions
   - Cite stewardship-over-sovereignty, rea-compute-commitment-primitive, cradle-to-grave-capability-gradient
2. - [ ] Write `pillar-bundle-split-runbook.md`:
   - "When you split a pillar from elohim-app into its own bundle, you must update:"
   - Codegen target paths (lamad:codegen, schema:codegen:ts, manifest:codegen)
   - tsconfig path aliases (bidirectional — both bundles)
   - Schema distribution targets (codegen-ts.mjs GENERATED_OUTPUT_DIRS)
   - Pre-push gate paths (.husky/pre-push virtual gates)
   - Spec file imports (the 7 we found post-B18)
   - Lint manifest regeneration
   - Storybook config (if pillar has UI primitives)
   - A2o feature scenarios (URL routes change)
   - Capture the patterns from THIS lamad split as the foundational case study
   - **Common gotchas captured from preceding integrations** (subsection):
     - **Bundle package.json minimum scripts.** Every new bundle ships with `build`, `start`, AND `test` scripts. The peer-OAuth-portal `app/imagodei-portal/package.json` shipped without `test`, meaning husky and the worker scan could not discover its 9 standalone-resolver tests. Add the missing script to the runbook scaffold template.
     - **Pnpm lockfile reconciliation.** When `pnpm-workspace.yaml` gains a new bundle entry, the lockfile must be regenerated. `pnpm install --lockfile-only` is the fast path (~45s on this monorepo). Caught during the design/peer-oauth-portal merge; required because both branches added different new workspace entries (app/lamad and app/imagodei-portal).
     - **Build-artifact tracking inconsistency.** `app/elohim-elements/elohim-core/dist/custom-elements.json` is git-tracked but `app/elohim-elements/elohim-imagodei/dist/` is gitignored. The runbook MUST take a position: pick one policy, propagate it, and call out that builders depend on the artifact being present locally (the elohim-imagodei tests 404 on `custom-elements.json` until `pnpm --filter elohim-imagodei run build` runs first).
     - **Test-spec orphaning on Lit-wrapper refactor.** When an Angular component is rewritten as a thin Lit wrapper (the elohim-imagodei pattern: `LoginComponent`, `AuthCallbackComponent`, `EprLinkComponent`), the old spec asserting `ngOnChanges`/`OnChanges`/internal-state behavior STAYS unless explicitly retired. The pre-push gate catches it; the integrator pays the cost. Convention: a refactor that removes a lifecycle hook MUST update or replace the spec in the same commit.
     - **Route-count specs as canaries of bundle splits.** `app.routes.spec.ts` asserting `routes.length === 14` continued to assert the pre-split count after lamad moved out (effc26e04). Specs that count entities or list routes are canaries — they belong in the bundle-split checklist as files to audit.
     - **Clippy regressions from upstream merges.** When pulling origin/dev forward, a clippy `useless_conversion` in `doorway/src/server/http.rs:1257` (introduced by 542ca8f0b, fixed in 8c66fa3ca) blocked the elohim-edge Docker build for build 1002. Husky pre-push catches it locally; the convention is to fix-before-push rather than push-and-watch-CI-burn.
3. - [ ] Update `README.md` index in `genesis/docs/architecture/`
4. - [ ] Commit Slice 2.7 milestone

**Acceptance signal:** Future pillar split (shefa, qahal as own bundles) can follow the runbook without re-deriving the lessons.

---

## Wave 3 — Cutover (1 agent, sequential)

**Agent:** `claude` (full context needed — coordinates verification across all slices)
**Scope:** Final cutover and verification.
**Duration:** ~2 hours.

### Steps

- [ ] **Step 1: Verify each Wave 2 slice landed cleanly**

  ```bash
  # Each slice committed its milestone. Verify the milestones exist.
  git log --oneline | grep -E "Slice 2\.[1-7]"
  # Expect 7 milestone commits.
  ```

- [ ] **Step 2: Remove cross-pillar aliases from app/lamad/tsconfig.json**

  Delete these alias entries:
  ```
  "@app/elohim", "@app/elohim/*",
  "@app/imagodei", "@app/imagodei/*",
  "@app/qahal", "@app/qahal/*",
  "@app/shefa", "@app/shefa/*",
  "@app/generated/*",
  "@app/doorway", "@app/doorway/*",
  "@app/avodah", "@app/avodah/*"
  ```

  Keep: `@app/lamad/*` (lamad's own internal alias) and `@elohim/*` library aliases.

- [ ] **Step 3: Verify app/lamad builds STANDALONE**

  ```bash
  pnpm --filter lamad build 2>&1 | tail -20
  ```

  Expected: clean build, no "Could not resolve" errors. If any import fails, the relevant Wave 2 slice missed something.

- [ ] **Step 4: Verify app/elohim-app still builds (bidirectional check)**

  ```bash
  pnpm --filter elohim-app build 2>&1 | tail -20
  ```

- [ ] **Step 5: Run the full pre-push suite**

  ```bash
  pnpm run schema:test
  pnpm --filter holochain-seeder test
  pnpm --filter elohim-app test
  pnpm --filter lamad test
  ```

- [ ] **Step 6: Update canon docs that reference cross-pillar imports**

  Search `genesis/docs/architecture/` for any reference to the transitional aliases; update to reference the new SDK boundary docs.

- [ ] **Step 7: Commit the cutover milestone**

  ```bash
  git add app/lamad/tsconfig.json
  git commit -m "feat(lamad): bundle independence — cross-pillar imports retired

  Closes the cross-pillar import cleanup sprint (plan:
  genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md).
  Wave 1 produced the 159-import disposition manifest; Wave 2 migrated
  each import to its substrate-correct home (@elohim/service for shared
  services, elohim-core for UI primitives, @elohim/identity for auth
  primitives, @elohim/rea-runtime for REA primitives, @elohim/storage-
  client for wire types, doorway HTTP / EPR resolution for cross-pillar
  data flow). Wave 3 retired the transitional aliases.

  Lamad now builds as a standalone bundle without source-reach into
  elohim-app. The Elohim SDK boundary is documented at
  genesis/docs/architecture/elohim-sdk.md; the pillar-bundle-split
  runbook at .../pillar-bundle-split-runbook.md captures the patterns
  so the next pillar split (shefa, qahal, etc.) can follow without
  re-deriving."
  ```

- [ ] **Step 8: Push**

  Standard `git push origin dev` (husky validates).

### Wave 3 acceptance signals

- `app/lamad/tsconfig.json` has zero `@app/<other-pillar>/*` aliases.
- `pnpm --filter lamad build` succeeds.
- `pnpm --filter elohim-app build` succeeds.
- Full pre-push suite passes.
- 4-canon-doc set is internally consistent (sovereignty + REA primitive + cradle-to-grave + new SDK doc).
- Pillar-bundle-split runbook is exhaustive enough that someone could split shefa as its own bundle by following it.

---

## File Structure (post-cutover)

```
app/
├── elohim-app/         (residual monolith — most pillars still here; shrinking with each split)
│   └── src/app/
│       ├── elohim/     (cross-cutting — migrating out via Slice 2.1)
│       ├── imagodei/   (migrating out via Slice 2.3)
│       ├── qahal/      (UI primitives migrating to elohim-core via Slice 2.2)
│       ├── shefa/      (migrating out via Slice 2.4)
│       ├── doorway/    (consumed by elohim-app as a pillar)
│       └── ...
│
├── lamad/              (standalone SPA bundle, no source-reach to elohim-app)
│   ├── src/app/
│   └── tsconfig.json   (only @app/lamad/* + @elohim/* aliases remain)
│
├── elohim-elements/
│   ├── elohim-core/    (Lit element library — UI primitives + register)
│   └── elohim-imagodei/(domain Lit elements — peer-OAuth portal etc.)
│
└── elohim-library/
    └── projects/
        ├── elohim-service/        (existing — Slice 2.1 target for shared services)
        ├── elohim-identity/       (NEW — Slice 2.3 target for auth primitives)
        ├── elohim-rea-runtime/    (NEW — Slice 2.4 target for REA primitives)
        └── graphos/               (existing — Library A/B stories)

elohim/sdk/storage-client-ts/      (existing — Slice 2.5 target for wire types)
```

---

## Coordination Protocol

### Wave 1 → Wave 2 handoff

Wave 1's manifest is the input contract. Operator reviews the "operator input needed" section before Wave 2 dispatches. If library-structure decisions change (e.g., fold @elohim/identity into @elohim/service), update Wave 2's slice scope before dispatching.

### Wave 2 parallel execution

All 7 slices are file-disjoint:
- 2.1 → `@elohim/service` library + L-classified consumers
- 2.2 → `elohim-core` Lit library + C-classified consumers
- 2.3 → `@elohim/identity` library + I-classified consumers
- 2.4 → `@elohim/rea-runtime` library + R-classified consumers
- 2.5 → `@elohim/storage-client` + S-classified consumers + codegen distribution
- 2.6 → H/E consumers (substrate-API calls)
- 2.7 → documentation (no code)

**Conflict risk:** the public-api.ts of `@elohim/service` is touched by Slice 2.1 only. Other libraries each have their own public-api.ts. Slice 2.2's storybook config is independent. Slice 2.7 is docs.

Each slice agent commits independently. Wave 3 verifies cohesion.

### Wave 3 coordination

Wave 3 agent verifies each slice's milestone commit exists before proceeding. If any slice is incomplete (e.g., couldn't migrate a specific service), Wave 3 surfaces the gap to operator instead of pushing forward with a broken tsconfig.

---

## Self-Review

After writing this plan, the author checks:

- [x] **Spec coverage:** Every disposition code (L/C/S/I/R/H/E/D/X) has a Wave 2 slice or explicit "handled in Wave 1 manifest" treatment.
- [x] **Placeholder scan:** No "TBD" or "TODO." Every task names specific files and commands.
- [x] **Type consistency:** `@elohim/service`, `elohim-core`, `@elohim/identity`, `@elohim/rea-runtime`, `@elohim/storage-client` library names used consistently throughout.
- [x] **Parallelism real:** File scopes documented as disjoint; coordination points (public-api.ts per library) named.
- [x] **Open question coverage:** The library-structure question (new lib vs fold) named explicitly in Slice 2.3 + 2.4 with default choice + escalation path.

---

## Why This Plan Matters Beyond Lamad

The lamad split is the **first** of many planned pillar splits. The pillar-EPR decomposition design (`2026-05-25-pillar-epr-decomposition-design.md`) names: lamad, shefa, qahal, avodah, imagodei, account, doorway as future independent bundles. Each split will face the same cross-pillar import problem.

**Slice 2.7's deliverables** (`elohim-sdk.md` + `pillar-bundle-split-runbook.md`) are the durable artifacts. After this sprint:

- Future pillar splits follow the runbook instead of re-deriving the lessons.
- The Elohim SDK has a clear, documented surface that elohim-native developers can build against.
- The "AI deployment maturity" framing the protocol aims for has a concrete, navigable substrate — agents new to the codebase can read `elohim-sdk.md` and understand what to build against.

This sprint is therefore **canon-tier work disguised as cleanup**. The library boundary that emerges here is the SDK the rest of the protocol will use for years.

---

## Execution

After this plan is approved:

1. **Dispatch Wave 1** via the `cartographer` subagent (Opus). Read manifest.
2. **Operator reviews** the library-structure questions surfaced in Wave 1.
3. **Dispatch Wave 2 slices in parallel** via `superpowers:dispatching-parallel-agents` — one prompt per slice with the manifest as input.
4. **Watch for slice milestones** in `git log --oneline`. Each slice reports its acceptance signal when complete.
5. **Dispatch Wave 3** once all 7 milestones are present.
6. **Push** the cutover milestone to dev.

Each slice agent should be briefed with:
- The relevant section of this plan (their slice + the disposition manifest)
- The relevant canon docs (sovereignty + REA primitive + cradle-to-grave + the in-flight elohim-sdk.md)
- File paths for their migration scope

---

## References

- B0 audit: `genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md`
- Pillar-EPR design: `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`
- Canon: `genesis/docs/architecture/stewardship-over-sovereignty.md`, `.../rea-compute-commitment-primitive.md`, `.../cradle-to-grave-capability-gradient.md`
- Capability profile element contract: `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`
- Z.D spec (mentions REA library as Wave 2 target): `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md`
- Z.D implementation plan (related sprint, runs separately): `/projects/.claude-config/plans/abundant-wandering-lemon.md`

---

## Closing note

This is **a planned sprint** — not an emergency cleanup. Schedule it after Z.D Phase 1 substrate work lands (so the new SDK already has the REA compute-commitment primitive available). Treat it as the **graduation of the lamad split from "MVP cross-bundle reach" to "real bundle independence."** Run it when the team can give the parallel slice agents focused attention. Don't squeeze it between other sprints.

When it completes, the Elohim protocol has a real SDK and a real runbook for pillar splits. That's the milestone.
