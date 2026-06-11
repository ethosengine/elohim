---
title: SDK Framework-Free Core Entrypoints — type:module + /core subpath exports (pre-Phase-4 gate)
id: sdk-core-entrypoints-plan
status: landed
class: process-meta
process_subdomain: schema-sdk
sprint: unranked — born 2026-06-10; gates arc Phase 4 (rea-runtime CommitmentService). Packaging-only — operational concern, no storage schema, no DHT entities.
cites:
  - elohim-sdk-architecture | elohim-sdk | sha256:7d1a9b09f3c6592d | path: genesis/docs/architecture/elohim-sdk.md
  - genesis/docs/superpowers/plans/2026-05-18-sdk-boundary-clarification.md
  - che-network-agency-arc-design | parent arc — Task 1.2 empirically produced the two module-resolution failure modes this packaging pattern retires; Phase 4 (rea-runtime) is gated on this plan | sha256:ede3841e83bc2b65 | path: genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
  - genesis/data/timeline/backlog/elohim-identity-type-module-esm-interop.md
  - lit-wc-component-layer-pivot | directional lesson — the component layer already left Angular-library packaging; core entrypoints continue that direction for SDK services | sha256:b46d2c8b087f04ff | path: genesis/docs/content/elohim-protocol/history/2026-05-06-lit-wc-component-layer-pivot.md
informed-by:
  - genesis/docs/architecture/elohim-sdk.md
---

# SDK Framework-Free Core Entrypoints — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or
> superpowers:executing-plans, task-by-task, two-stage review per task.

**Goal:** Give the five-library SDK a sanctioned **Node-consumable, framework-free entrypoint
pattern** — `"type": "module"` + a `./core` subpath export per library that needs one — so the
arc's Stage A/C clients (and every future non-Angular consumer) import cleanly instead of
copying Task 1.2's CJS/ESM interop shim and deep-module imports.

**Why now (sequencing):** arc Phase 4's `CommitmentService` in `@elohim/rea-runtime` has the
identical Node+browser consumer set and must be **born with this pattern**. This plan is the
gate; it is small.

**Orientation:** process-meta (schema-sdk; operational packaging — no storage, no DHT, source of truth unaffected). Lexical prior-art (semantic lens unavailable —
degraded, stated): composes from the SDK five-libraries canon (`elohim-sdk.md`) and the SDK
boundary-clarification plan; the Lit-WC pivot history is the directional lesson (the component
layer already left Angular-library packaging — core entrypoints continue that direction for
services). Evidence base: arc Task 1.2 (commit 0026de6b1) empirically proved both failure modes
(named-import failure under tsx ESM; default-import failure under cucumber CJS) and the
workarounds this plan retires.

---

## Task 1: Decide + document the pattern in the SDK canon

**Files:** `genesis/docs/architecture/elohim-sdk.md` (managed surface — cite tooling applies).

- [x] Add a compact section "Node-consumable core entrypoints": each library that hosts
      framework-free modules declares `"type": "module"` and an `exports` map with `./core`
      (framework-free only — no Angular imports transitively) beside the Angular root entry;
      consumers outside Angular import ONLY `@elohim/<lib>/core`. State the rule for new
      libraries: born with the pattern when any non-Angular consumer is foreseeable.
- [x] Note the two empirical failure modes (from arc 1.2) the pattern prevents, one line each.
- [x] `cite-gen.py --seal` clean on the edited doc. Commit.

## Task 2: Implement in `@elohim/identity` (the proven consumer set)

**Files:** `app/elohim-library/projects/elohim-identity/package.json`, new `src/core.ts`,
`ng-package`/tsconfig surfaces ONLY if the Angular build requires.

- [x] Add `"type": "module"` + `exports` map: root → existing public-api path (verify what the
      Angular consumers resolve today — elohim-app consumes via tsconfig path alias
      `app/elohim-app/tsconfig.json:25-26`; confirm the alias bypasses `exports` and therefore
      cannot break), `./core` → `src/core.ts` (or built equivalent — match how a2o resolves the
      source via its alias; keep BOTH the alias path and the exports map coherent).
- [x] `src/core.ts` exports ONLY framework-free modules: `doorway-session-client` (+ its types),
      any models with zero Angular imports (verify transitively — a grep for `@angular` in the
      import closure of core.ts must be empty; add that as a unit test so the boundary is
      enforced, not aspirational).
- [x] Identity suite still green: `cd app/elohim-library && pnpm exec vitest run --config
      vite.config.ts projects/elohim-identity` (67 tests).
- [x] Angular consumer unaffected: run the elohim-app test subset that imports `@elohim/identity`
      (locate via grep; run those spec files under the app's vitest) — green. If the app consumes
      source via alias only, additionally `pnpm run lint` on the importing files for module-res
      sanity. Commit.

## Task 3: Collapse the a2o interop shim

**Files:** `genesis/a2o/src/framework/api/doorway-client.ts`, `genesis/a2o/tsconfig.json`.

- [x] Replace the `namespace.default ?? namespace` interop and deep-module import with a plain
      named import from `@elohim/identity/core`; update the a2o tsconfig alias to match;
      delete the interop comment block.
- [x] Re-verify all three pipelines (the 1.2 evidence set): `pnpm typecheck` (0),
      `pnpm test:unit` (107), `pnpm exec cucumber-js --dry-run --tags '@e2e'` (exit 0,
      514 scenarios). Commit.

## Task 4: Pre-pave `@elohim/rea-runtime` + sweep the remaining libraries

**Files:** `app/elohim-library/projects/elohim-rea-runtime/package.json` (+ a stub core.ts only
if it already has framework-free modules), checklist additions to the canon section.

- [x] `@elohim/rea-runtime`: add `"type": "module"` + `./core` export now (empty-or-minimal core
      is fine — Phase 4's CommitmentService lands INTO it); verify its existing tests/build.
- [x] Audit `@elohim/service`, `@elohim/storage-client`, `elohim-core` (Lit): which already have
      honest module metadata (storage-client is plain tsc — likely fine), which would break
      consumers if flipped. Output = a one-line verdict per library appended to the canon
      section's checklist — flip ONLY where a consumer exists today; document the rest.
- [x] Full gate: a2o suites + identity suite + `pnpm run lint` on touched packages. Commit.

## Out of scope

The auth wire-contract work and Angular auth migration — operational, no storage concern — (sibling plan
`2026-06-10-auth-wire-contract-completion-plan.md`); any ng-packagr dist/build redesign; the
SDK scaffolding CLI (its spec's trigger — next domain manifest split — has not fired).


## Execution log (2026-06-10, subagent-driven, two-stage reviews — plan LANDED)

- **T1** canon §4.1: e68883a94 (controller-reviewed).
- **T2** identity /core: b3edef551 — 70 tests incl. boundary walker w/ positive control; 254 elohim-app
  consumer tests green; ng build green. Review ✅/APPROVED (2 Minors → T4 caveats).
- **T3** a2o shim collapse: 8378b693f — alias removed entirely (exports map single-homed); caught+fixed
  latent T2 gap (.js-suffixed re-exports required under Node16). Review ✅/APPROVED (deviation justified).
- **T4** rea-runtime pre-pave + audit + canon close: 44c06a3d7 + fb6c112b8 — 69 tests; honest audit
  (storage-client NOT flipped: CJS emit; elohim-core already honest). Review ✅/✅.
- **Carried forward:** LOW latent — rea-runtime `protocol-event-types.ts:16` extensionless type import
  trips TS2835 on first node16 /core consumer (arc Phase 4); one-line fix then. Silent root-config test
  gap filed: backlog/rea-runtime-specs-silent-skip-root-config.md.

## Self-Review

Composes from canon (pattern lands IN elohim-sdk.md, not a parallel doc); evidence-based (both
failure modes already reproduced in 1.2); byte-safe for Angular consumers (alias-resolution
verified before exports flip); enforced boundary (the no-@angular-in-closure unit test);
sequenced as the Phase 4 gate.
