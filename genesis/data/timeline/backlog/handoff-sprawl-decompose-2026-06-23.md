---
id: "backlog-handoff-sprawl-decompose-2026-06-23"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Handoff-doc sprawl: restore the decompose discipline (README de-blessed) + inventory the residual docs for harvest + a2o navigation-track plan"
slug: "handoff-sprawl-decompose-2026-06-23"
written: "2026-06-23"
author: "deployment shakeout sprint (overnight, autonomous) — handoff-sprawl scout + a2o-coverage scout"
status: "open"
priority: "medium"
tags: [memory, decompose, handoff, hygiene, discipline, a2o, navigation, console-capture, shakeout]
relatedNodeIds:
  - backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag
cites:
  - .claude/handoffs/archive/README.md
  - .claude/skills/agentic-developer/SKILL.md
  - genesis/a2o/scripts/look.ts
  - genesis/a2o/src/framework/utils/console-filters.ts
  - genesis/a2o/features/browser/navigation-browser.feature
---

# Handoff-doc sprawl → decompose discipline (2026-06-23)

## Why this exists

A scout found ~1,670 lines of handoff-doc sprawl across 16 files (repo root +
`.claude/handoffs/`). There is **no culprit skill** — the `handoff` skill was retired
2026-06-11 (`df250665f`); the sprawl is an *unowned convention* that the archive README
**blessed** ("keep ≤4 at root, archive the rest") — a volume gate, never a decompose gate.
The decompose discipline already exists (`agentic-developer` Close step 5, the
No-Dumping-Grounds law) but is scoped to `.claude/shifts/` plans, not handoffs.

## Done this session

- **De-blessed the README** (`.claude/handoffs/archive/README.md`): archive is now a
  temporary, operator-purgeable holding pen pending decompose — not a permanent store.
- **Removed 10 superseded docs** whose signal is already durable (git history for tracked;
  operator-declared-stale / commit-superseded for untracked):
  - Tracked (recoverable via git): `HANDOFF-2026-06-17-fbootstrap-deploy-gate.md`,
    `HANDOFF-2026-06-17-upstream-tx5-transport-pin.md`,
    `HANDOFF-2026-06-18-conductor-leak-rca-reopened.md`,
    `.claude/handoffs/2026-06-20-attestation-cleanup-deliverable-handoff.md`,
    `.claude/handoffs/archive/HANDOFF-2026-06-17-conductor-leak-hunt.md`.
    (Conductor-leak/503/tx5/attestation signal lives in memory:
    `project_storage_metrics_surface_and_leak_verdict`, `project_tiered_quilt_unblock_state`.)
  - Untracked, operator-declared-stale or commit-superseded:
    `archive/HANDOFF-2026-06-16-cid-landed-503-staged.md`,
    `archive/OVERNIGHT-DELIVERY-HANDOFF-2026-06-13.md`,
    `archive/OVERNIGHT-HANDOFF-2026-06-16-break-503-and-genesis-shakeout.md`,
    `archive/SPRINTER-HANDOFF-2026-06-14.md`,
    `2026-06-19-elohim-facings-crate-extraction-handoff.md` (superseded by code-review
    commits `ab5ca0d57..c64b39367`).

## Residual docs — KEPT for a supervised harvest (live signal)

These carry still-open or actively-relevant signal; a supervised decompose pass should
harvest each into the named home, then remove the file:

| Doc | Open signal → harvest target |
|---|---|
| `.claude/handoffs/HANDOFF-2026-06-17-doorway-metrics.md` (relocated from root 2026-06-23) | doorway `/metrics` P2 (M1–M5) — **largely landed** (`25bc75b1b`,`e53709967`); confirm no residual P3 items, then remove. |
| `2026-06-18-resilience-cards-dual-doorway-sprint-RESULT.md` + `-handoff.md` | THE FORK: `GET /auth/me` 401 from dev is a **known structural item** (session/key-population path; p2p-design-gate + security-owned) — this is the root of the live "steward auth isn't working" symptom; gated on integrator dev-merge + reseed + jemalloc deploy. Also: seeder still stamps `p2pPublishedAt` (`seed-sqlite.ts:903`) instead of the honest ingest `dhtAnchorHash`. → migrate to a backlog item. |
| `2026-06-21-shakeout-shift-handoff.md` + `2026-06-21-visual-verification-map-for-shakeout-shift.md` | The pre-existing RED floor punch-list (storage `observe_kind` is `#[cfg(test)]` → `observed_kinds()==[]` cross-crate; elohim-app gate pre-existing-red → `--no-verify` is the only frontend dev-push path). Partly in memory (`feedback_pvc_deferral_hides_gate_debt`). Visual-verification-map names which surfaces are deployed-app-verifiable — read it before the next `/shift`. |

## Recommended (operator-gated): close the scope gap

Extend the `agentic-developer` No-Dumping-Grounds law (Close step 5) to name **handoff
documents** explicitly in its decompose scope ("for each plan/spec/**handoff** this shift
concluded …"), so the discipline is enforced at the tool boundary rather than relying on
the README. Editing a core skill overnight is out of scope for an unsupervised run.

---

# a2o navigation + console-capture: the parallel-track plan (Track C deliverable)

**The gap that let the deployment bugs ship:** `genesis/a2o/scripts/look.ts` *does* capture
`httpErrors` (4xx/5xx) and console errors, but the BDD navigation scenarios don't fail on
them — `Then the page should load successfully` only checks `<body>` is visible, and
`assertNoConsoleErrors` runs through `console-filters.ts`/`isSpaRoutingNoise`, which
**deliberately filters out 404s/403s**. So a route can WASM-404, doorway-503, and render the
not-found component while the test still passes. The `deep-link-delivery.feature` scenarios
already fixed this by asserting on rendered `data-testid` elements — that's the pattern to
propagate.

**DO NOT add a blanket "no httpErrors" gate** — several live signatures are out of this
sprint's scope (manifesto `403` may be the *intended* commons-reach gate; `nav-context 404`;
`appreciations 404`; the WASM-Harbor side is operator/CI). A blanket gate locks genesis red.
Scope each new assertion to the *specific route + signature* a fix resolved.

**Five parallel tracks** (each gets a scoped navigation scenario that asserts on the specific
fixed signature, run against alpha *after* the fix deploys):

1. **EPR-routing / SPA-fallthrough** — `/identity*` and root deep-links render the SPA shell,
   not a conductor/JSON 404. (`/identity` fix landed this sprint; locked by the doorway unit
   test `shakeout_service_path_identity_narrowed_to_did`.)
2. **Asset / WASM** — on alpha/prod no `/wasm/elohim-cache-core/...` request fires
   (`preferWasm` wired this sprint). Assert: no httpError for that URL.
3. **Map / WebGL** — `/map` degrades gracefully: `data-testid="map-error"` present, no
   uncaught pageerror (map fix landed this sprint).
4. **API-503** — `custodians/metrics/recommendations` returns an honest 404, never a panic-503
   (storage panic fix landed this sprint).
5. **Operator portal / auth** — `/threshold/*` vs `/dashboard` confusion; steward `/auth/me`
   (blocked on the known structural item above — do not author until that lands).

Authoring these E2E scenarios was deferred from the overnight run: deploy-timing races,
headless-login can't auth against alpha, and "a gherkin parse error aborts the whole run"
make new E2E unsafe to land unsupervised. Author them in a supervised session, scoped as
above, once the fixes are confirmed live.
