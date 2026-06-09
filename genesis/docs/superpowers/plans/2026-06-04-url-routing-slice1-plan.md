---
title: URL & Routing Contract — Slice 1 (alpha green) Implementation Plan
id: url-routing-slice1-plan
status: Draft
class: protocol-canonical
topic: [epr-routing, spa-fallback, doorway, lamad-bundle, deep-links, base-href]
cites:
  - pillar-epr-decomposition-design | the decomposition seed this plan refines — slice-1 routing legs implement its pillar/bundle URL boundaries | sha256:8029079cea758380 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - substrate-shakeout-epr-delivery-sprint | the prior delivery sprint this slice continues — its open render-verified acceptance is satisfied by leg D | sha256:086e3437eb475995 | path: genesis/docs/superpowers/plans/2026-05-29-substrate-shakeout-epr-delivery-sprint.md
  - doorway-dispatch-registry-fallback-and-vocabulary | the registry-fallback gotcha to read before touching doorway dispatch — leg A modifies the same dispatch surface | sha256:8adde339010ac508 | path: genesis/docs/content/elohim-protocol/history/2026-06-02-doorway-dispatch-registry-fallback-and-vocabulary.md
refines: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
domain: D8
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
sprint: Sprint-5
---

# URL & Routing Contract — Slice 1 (alpha green)

Implements §12.6 **Slice 1** of the pillar-EPR decomposition spec: make
`alpha.elohim.host/lamad/path/foundations-christian-technology` actually render, and stop the
lamad bundle minting doubled `/lamad/lamad/…` URLs. Drains gap-items **#12-1, #12-2, #12-3,
#12-6 (partial: bundle-local literals + pragmatic resolver patch), #12-10** from
`.claude/memory-kit/gap-items/specs__2026-05-25-pillar-epr-decomposition-design.json`.

**Execution vehicle:** authored and run as a `/workflows` orchestration (four parallel legs →
per-leg gates → adversarial review). This document is the auditable scope record; the workflow
script is the executable form.

**Semantic lens note:** MemPalace recall unavailable in this session (per-subagent MCP; index
flagged stale) — degraded to lexical floor. Prior plans composed from: substrate-shakeout EPR
delivery sprint (delivery surface), landing-page-epr-dual-doorway (projection seeding),
cross-pillar-import-cleanup (bundle-split discipline). The 2026-06-02 history record
*doorway-dispatch-registry-fallback* read for the registry-fallback gotcha before touching
dispatch.

## Legs (disjoint file sets; run in parallel)

### Leg A — doorway + views + schema (gap #12-1)
1. `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json`: add `spaFallback` (boolean,
   default true) per CONVENTIONS.md.
2. `elohim/elohim-views/src/projection.rs` `EprProjectionView`: add `spa_fallback: bool` with
   `#[serde(default = "default_true")]` (camelCase wire: `spaFallback`).
3. Schema contract test (`elohim/elohim-storage/tests/schema_contract.rs`) covers the field.
4. Regenerate ts-rs bindings (`cargo test export_bindings`) — additive diff only on
   `elohim/sdk/storage-client-ts/src/generated/EprProjectionView.ts`.
5. `doorway/doorway-service/src/server/http.rs`: ROUTE/ASSET helper (final segment contains no
   `.`) + `derive_app_subpath` gains the projection's `spa_fallback`: strip mount → empty →
   `entry_file`; ROUTE ∧ spa_fallback → `entry_file`; else verbatim. Unit tests consume the
   shared vector fixture.

### Leg B — storage safety net (gap #12-2)
`elohim/elohim-storage/src/http.rs` `handle_app_request`: on requested-file miss (BOTH the
extraction path and the cache-hit path), ROUTE → serve the bundle's own `index.html` with
`X-SPA-Fallback: 1` + correct content-type; ASSET → existing 404 JSON unchanged. Same
ROUTE/ASSET rule; tests consume the same shared vector fixture.

### Leg C — lamad bundle de-literalization (gap #12-6 partial)
Rewrite every bundle-internal `'/lamad…'` router literal to its base-href-relative form
(`'/lamad/path'→'/path'`, `'/lamad'→'/'`, `'/lamad/explore'→'/explore'`, …) across ~20 sites:
content-viewer `navigateToPath`, path-navigator templates, path-overview + lamad-home
`PATH_ROUTE`, lamad-layout `isHomePage` (router.url is post-base-strip: compare `'/'`),
mission-card, graph-explorer, lamad-not-found, search/me/human routes. Update co-located specs
(route-count/route-shape canaries per the bundle-split runbook §4.4). Pragmatic resolver patch:
`epr-resolver.service.ts:193,204` + `eprToRoute()` (`epr-ref.ts:159`) `'/lamad/path'→'/path'`
with `TODO(#12-6)` pointing at the Slice-2 claims rewrite. PRESERVE uncommitted in-flight
changes on this branch — edit on top, never revert.

### Leg D — a2o render-verified scenarios (gap #12-10)
Author scenarios per §12.5 in `genesis/a2o/features/` (existing lamad/content conventions +
tagging): cold shared path URL renders · deep step link · legacy doubled URL shows designed
not-found · asset miss stays 404 JSON. (`/epr/{id}` scenario deferred to Slice 2 with its
implementation.) Steps under `genesis/a2o/steps/lamad/` following the in-flight
`epr-link-navigation.steps.ts` patterns.

## Shared fixture (pre-created before legs launch)
`elohim/sdk/fixtures/spa-route-discrimination.vectors.json` — the ONE test-vector table both
Rust implementations consume (`include_str!` via relative path). The two-layer drift guard from
§12.2.

## Gates (per leg)
- **A:** `RUSTFLAGS="" cargo nextest run` + `clippy -- -D warnings` + `fmt --check`
  (doorway-service, pool slot `family/shift/doorway__doorway-service`).
- **A+B:** storage workspace `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo nextest run`
  + `cargo test export_bindings` (pool slot `family/shift/elohim__elohim-storage`). Runs after
  BOTH A and B land (export_bindings compiles the storage workspace).
- **C:** lamad vitest + eslint.
- **D:** cucumber dry-run / feature lint per a2o framework.
- **All:** code-reviewer adversarial pass over the full diff; verification-before-completion.

## Out of scope (Slice 2/3 — already gap-tracked)
Universal `/epr/{id}` resolver (#12-5, #12-8) · BundleRouteContext claims rewrite (#12-6 full)
· routeClaims manifests (#12-7) · reserved-prefix validation (#12-4) · pushState card-flip
(#12-9) · EPR-derived menu (§7.5).

## Done =
All four gates green + canonical URL renders the path overview locally (`hc:start:seed` +
`pnpm look /lamad/path/foundations-christian-technology`) + commit proposal presented to the
operator (tree carries unrelated in-flight work; commit boundaries are operator's call).
