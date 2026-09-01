---
title: "EPR Slice 1 — lens-complete /epr/{id}: demote the claims-302, focal epr-composite renderer, Open-in-pillar lens"
id: epr-slice1-lens-complete-resolver-plan
status: Draft
class: protocol-canonical
domain: D8
sprint: slice-1
cites:
  - "lens-complete-epr-resolution-four-leg-coupling-design | the parent design; this plan implements Slice 1 (demote claims-302 + focal epr-composite render + Open-in-pillar lens) | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md"
  - "epr-acquisition-pull-queue-design | owns the typed-relation ClusterClosure (HELD here, composed in Slice 3) + the value-leg provide-content substrate (HELD, Slice 2) — do not fork | sha256:fc4a0cdd9828a377 | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md"
  - "epr-slice0-raw-node-surface-plan | the prior slice (raw-node surface); this builds on its EprSubview dispatch + the shell /epr routing | sha256:4487dd4b33bb4f4f | path: genesis/docs/superpowers/plans/2026-06-07-epr-slice0-raw-node-surface-plan.md"
---

# EPR Slice 1 — lens-complete `/epr/{id}`

Realizes the design's core: `/epr/{id}` is the **lens-complete home** (focal content + its coupled
legs), and the pillar mount is one **lens**, not a forced 302. Operator decision 2026-06-08:
**demote the claims-302** — stop bouncing claimed `/epr/{id}` to the pillar; render the lens-complete
viewer + an "Open in {pillar}" affordance.

## Substrate ledger (investigation 2026-06-08) — what EXISTS vs what this slice BUILDS

Most of the lens-complete viewer ALREADY EXISTS — the shell's `ContentViewerComponent`
(`@app/lamad/components/content-viewer`, mounted at `/epr/:resourceId`) already renders:
- **Focal content** by `contentFormat` via `RendererRegistry` (`renderer-registry.service.ts`) — markdown,
  gherkin, html5-app, sophia(+aliases). ✓ REUSE
- **Knowledge leg** — `<app-epr-relationships-panel>` over `EprRelationship` (typed relations). ✓ REUSE
- **Governance leg** — governance tab + reach badge + challenges/discussions. ✓ REUSE
- **Value leg** — stewardship panel (PARTIAL: renders if data exists; the `provide-content` action +
  scorer arm are unbuilt → **HELD, Slice 2**, acquisition gap-items #7–9).

What this slice must BUILD (all bounded, household-testable):
1. **Demote the claims-302** (doorway) — today a claimed commons type 302s `/epr/{id}` to its pretty
   mount (`dispatch_epr_universal` / `classify_epr_universal`, `doorway-service/src/server/http.rs`),
   so a claimed type NEVER reaches the lens-complete viewer. Make the Default subview ServeShell.
2. **Focal `epr-composite` renderer** (KEYSTONE) — the registry has NO `epr-composite` renderer, so a
   path (`contentType:path`, `contentFormat:epr-composite`) falls to the generic
   `!hasRegisteredRenderer` fallback ("Content format: epr-composite", ~content-viewer.html:266-272).
   Post-demotion, paths land here — they MUST render well, not as raw fallback (else demotion is a
   regression on the focal experience). Build a composite renderer (the path's sections→items as a
   navigable outline; item refs as `/epr/{ref}` links).
3. **"Open in {pillar}" affordance** — when the content type is CLAIMED, the viewer offers a link to
   the pretty pillar mount (the rich single-leg deep-dive). Compute the mount from the content type.

HELD (out of scope — own slices, partly blocked on new substrate; compose, don't fork):
- **Value-leg substrate** — `provide-content` REA action + scorer arm. Slice 2. (acquisition #7–9 OPEN)
- **Typed-relation closure walk** — `ClusterClosure` is DESIGN-ONLY (acquisition §5.1, gap #11 OPEN);
  the knowledge leg here uses the EXISTING `EprRelationship` head, not a transitive closure. The full
  closure walk is Slice 3 and COMPOSES the acquisition spec's resolver — do NOT fork it here.

## MAP / roadmap

D8 (Web2 Projection & Doorway) for the dispatch demotion + the shell viewer it serves; the law it
serves is D1 (the four-leg coupling). Testable on `household-nodes` (focus AVAILABLE); nothing here
needs `shem`/`alpha-cluster`. The value-leg + closure HELD items are the blocked legs.

## Tasks

### Task 1 — Doorway: demote the universal claims-302
**Files:** `doorway/doorway-service/src/server/http.rs` (`dispatch_epr_universal`, `classify_epr_universal`).
**Approach:** the Default subview must now resolve to `ServeShell` (render the lens-complete shell
viewer), NOT `RedirectToMount`. The `Raw` subview already ServeShell (Slice 0). Net: `/epr/{id}` always
serves the shell. Remove/retire the `RedirectToMount` arm for the universal address (and the now-unused
`claimed_mount_location` lookup in this path — the SHELL computes the Open-in-pillar mount itself, Task 3).
**Do NOT touch** the sitemap (§7.5) or the legacy `/lamad/resource/{id}` → `/epr/{id}` alias-302
(redirectTemplates — a different dispatch path); those stay.
**TDD:** invert the existing `classify_raw_subview`/claimed tests — `Default + claimed + commons →
ServeShell` (was RedirectToMount). Keep the parser tests.
**Verify:** doorway `cargo test --lib --bins` (pool slot, RUSTFLAGS=""), clippy -D warnings, fmt.

### Task 2 — Shell: focal `epr-composite` renderer (KEYSTONE)
**Files:** a new renderer under `app/lamad/src/app/renderers/` (or content-io plugin) registered for
`epr-composite`; register it in `content-io/content-io.module.ts` alongside the others.
**Approach:** render the composite body (the `contentBody` sections→items: chapter/unit → step items)
as a clean, accessible OUTLINE — title/description + each section with its items, each item ref a link
to `/epr/{ref}` (the universal address, via `eprToUniversalHref`). This is the focal lens of a path AS
an EPR — NOT a re-implementation of the full path player (that stays at the pillar mount, Task 3
affordance). `data-testid` on the root for a2o. a11y: headings + lists.
**TDD:** Vitest — given an epr-composite node (sections/items fixture), renders the outline + one
`/epr/{ref}` link per item; root testid present; degrades if body is empty/unparseable.
**Verify:** `pnpm --filter lamad` vitest + route-literal lint (use the minter, never a literal).

### Task 3 — Shell: "Open in {pillar}" affordance
**Files:** `content-viewer.component.{ts,html}` (the cross-pillar viewer).
**Approach:** when the node's `contentType` is a CLAIMED pillar type (path → `/lamad/path/{id}`), show
an "Open in {pillar}" affordance (plain cross-bundle href, never routerLink). Derive the mount from the
content type → pillar-mount mapping (reuse the bundle route context / claims mapping the app already
holds; if none is cleanly available, a minimal contentType→mount map with a NOTE + backlog capture is
acceptable — do NOT invent a new substrate surface). `data-testid` for a2o.
**TDD:** Vitest — claimed type → affordance present with the right href; unclaimed type → absent.
**Verify:** lamad vitest + route-literal lint.

### Task 4 — a2o: rewrite the 302 scenario → lens-complete render
**Files:** `genesis/a2o/features/lms/deep-link-delivery.feature` (the scenario at ~:54 "Universal EPR
address 302s a claimed type to its pretty mount") + steps + selectors.
**Approach:** that scenario asserts the OLD 302 — INVERT it: `/epr/{claimed-path}` now renders the
lens-complete viewer (the focal composite outline + the knowledge leg; assert the Open-in-pillar
affordance present), NO 302. Add render steps/selectors for the composite-outline + Open-in-pillar
testids. Keep the unclaimed `/epr/{id}` → shell-viewer scenario. Tag `@regression`. (Opus authors the
narrative.)
**Verify:** tsc + eslint; testid-sync (selectors match the Task 2/3 testids); no slashes in titles.

## Done when

- `/epr/{claimed-path}` serves the shell (no 302) and renders the lens-complete viewer: a well-formed
  focal composite outline (not the raw fallback) + the knowledge + governance legs + an "Open in
  {pillar}" affordance to the pretty mount.
- `/epr/{id}` for unclaimed types is unchanged (already lens-complete-ish).
- The pillar mount (`/lamad/path/{id}`) still works directly (the rich deep-dive).
- All gates green (doorway cargo; lamad vitest + route-literal; a2o tsc/eslint). `@regression` scenario
  inverted + passing-shaped.

## Out of scope (later slices)

- **Slice 2:** value-leg substrate — `provide-content` REA action + scorer arm (acquisition #7–9).
- **Slice 3:** typed-relation closure walk (`ClusterClosure`, acquisition §5.1 / #11) — the transitive
  neighborhood; composes the acquisition resolver. Today's knowledge leg uses the existing head
  relations, not the closure.
- **CID-canonical addressing** (design Slice 3) and the **process leg** (design Slice 2) — unchanged here.
