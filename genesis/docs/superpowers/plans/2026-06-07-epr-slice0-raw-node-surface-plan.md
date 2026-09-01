---
title: "EPR Slice 0 — raw-node surface + contentFormat exposure + View-as-Content repoint"
id: epr-slice0-raw-node-surface-plan
status: Draft
class: protocol-canonical
domain: D8
sprint: slice-0
cites:
  - lens-complete-epr-resolution-four-leg-coupling-design | the parent design this plan implements Slice 0 of (gap-items #1-3: raw surface + contentFormat exposure + View-as-Content repoint) | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - genesis/data/timeline/backlog/epr-routing-complementary-captures.md
---

# EPR Slice 0 — raw-node surface (`/epr/{id}/raw`)

Bounded, shippable first slice of `lens-complete-epr-resolution-four-leg-coupling-design`
(gap-items #1–#3). Resolves the operator-clicked contradiction: "View as Content" on a claimed
type (a path) round-trips `/epr/{id}` → `/lamad/path/{id}` → itself. After this slice, the EPR has
a real **raw-node surface** to view it AS an EPR. **No coupling-law change, no new entity** — all
Operational (C) over existing Category-A content.

## Why this scope (and not more)

- The lens-complete resolver, the process leg, and CID-canonical addressing are Slices 1–4 — they
  touch the substrate LAW and are out of scope here. Slice 0 only adds a surface + exposes an
  existing field + repoints one link. Each task is household-node testable (doorway + app + lamad);
  nothing needs `shem`/`alpha-cluster` (focus: AVAILABLE = household-nodes).
- MAP: implementation lands in **D8 (Web2 Projection & Doorway)** + the elohim-app shell's universal
  viewer; the design it serves is **D1**. Doorway server-gate honored: `/epr/*` already has a
  doorway dispatch arm (`dispatch_epr_universal`, http.rs:1728) — we EXTEND it, not add a proxy; the
  raw *view* is shell-rendered via SPA bootstrap (doorway never authors content); the raw *data*
  reuses existing storage endpoints (no new manifest route in this slice).

## Current-state anchors (verified 2026-06-07)

- `dispatch_epr_universal` (`doorway-service/src/server/http.rs:1728`) strips `/epr/` and takes the
  FIRST path segment as `id` (`:1730-1735`), so `/epr/{id}/raw` currently resolves identically to
  `/epr/{id}` — the `/raw` suffix is silently dropped. → must be detected.
- `HeadFacts` (`http.rs:1677`) carries `content_type` + `reach` only; `fetch_head_facts` (`:1772`)
  GETs `/db/content/{id}`. `contentFormat` exists on that payload (`experience-story-epr §4.3`) but
  is not deserialized. → add `content_format`.
- `classify_epr_universal` (`:1710`): claimed + anon-reach → `RedirectToMount`; else `ServeShell`.
- The shell owns `epr/:resourceId` (cross-pillar viewer) per `app/elohim-app/CLAUDE.md`.
- "View as Content" anchor lives in `app/lamad/src/app/.../path-overview/path-overview.component.html`
  (touched by `2dafbde72`); cross-bundle nav uses plain `href` / `EprNavService`, never `routerLink`.

## Tasks

### Task 1 — Doorway: `/epr/{id}/raw` forces ServeShell (never 302)
**Files:** `doorway-service/src/server/http.rs` (`dispatch_epr_universal`, `classify_epr_universal`).
**Approach:** parse the path tail after the id segment; when it is `raw` (i.e. `/epr/{id}/raw`),
short-circuit to `ServeShell` BEFORE the claimed-mount lookup — the raw surface must never 302 to a
pillar (that is the round-trip we're killing). Keep `/epr/{id}` (no suffix) behavior unchanged.
Reserve unknown suffixes → ServeShell (fail open to the shell) and log.
**TDD:** unit test on the pure classifier path — add a `raw_suffix: bool` (or a parsed
`EprSubview`) input; `dispatch raw → ServeShell` even when a claim exists; `no suffix + claimed →
RedirectToMount` unchanged. Extend the existing `classify_epr_universal` tests (`http.rs:~3905`).
**Verify:** `RUSTFLAGS="" CARGO_TARGET_DIR=…/doorway__doorway-service/dev cargo test --lib --bins classify_epr` ; clippy -D warnings ; fmt.

### Task 2 — Doorway: surface `contentFormat` on HeadFacts (rail for Slice 1, used by the raw view)
**Files:** `http.rs` (`HeadFacts`).
**Approach:** add `#[serde(default)] pub content_format: Option<String>` to `HeadFacts`. Tolerant
(None on absence — fail open). Do NOT change `classify_epr_universal` dispatch logic this slice
(format-driven dispatch is Slice 1); the field is additive — available for the raw view + logged on
the ServeShell-raw path.
**TDD:** deserialization test: a `/db/content` JSON with `contentFormat:"epr-composite"` → HeadFacts
.content_format == Some("epr-composite"); missing → None.
**Verify:** same cargo gate as Task 1.

### Task 3 — Storage: confirm/raise the raw-node read (resource + relations + provenance)
**Files:** `elohim/elohim-storage/src/http.rs` (+ `build_manifest()` only if a new route is needed).
**Approach:** the raw view needs: the ContentNode (`/db/content/{id}` — EXISTS), its typed/related
ids (`relatedNodeIds` is on ContentNode — EXISTS), and provenance (the notarizing commitment / author
/ dht_anchor_hash). FIRST check whether existing endpoints already return enough for a Slice-0 raw
view. If provenance (commitment id + author + anchor hash) is NOT already reachable for a content id,
add the THINNEST read (prefer extending `/db/content/{id}` response or a `?include=provenance`
param over a new route; if a new route is unavoidable, declare it in `build_manifest()` so the
registry auto-routes — no doorway arm). **Capture richer typed-relation-neighborhood (the full
closure walk) as Slice 1 — do not build it here.**
**TDD:** storage unit/contract test for whatever field/endpoint is added (schema_contract if a view
changes). If existing endpoints suffice, NO storage change — record that finding in the task notes.
**Verify:** `cargo test export_bindings` if a view changed; schema_contract test.

### Task 4 — App (shell): `/epr/:id/raw` raw-node viewer component
**Files:** `app/elohim-app/src/app/app.routes.ts` (add `epr/:resourceId/raw` under the universal
viewer), new `…/components/epr-raw-node/` (or a `raw` mode of the existing `epr-resolve-redirect` /
universal viewer).
**Approach:** a blank-slate, accessible component that fetches the node (Task 3 data) and renders the
EPR AS an atom: **CID/EntryHash**, contentType, **contentFormat**, blob ref, `relatedNodeIds`
(as links to their `/epr/{id}` addresses), and provenance (author, notarizing commitment, anchor).
`data-testid="epr-raw-node"` on the root (page-model legibility). No pillar chrome. This is the
Slice-0 inspector; the lens-complete legs (value/governance/process affordances) are Slice 1.
**TDD:** Vitest component spec — given a mock node, renders the CID + contentFormat + a link per
relatedNodeId + the provenance block; root testid present.
**Verify:** `pnpm --filter elohim-app test`; lint.

### Task 5 — App (lamad): repoint "View as Content" → `/epr/{id}/raw`
**Files:** `app/lamad/src/app/.../path-overview/path-overview.component.html` (+ its component/ts if
the URL is built there).
**Approach:** change the View-as-Content target from the round-tripping `/epr/{id}` to `/epr/{id}/raw`.
Cross-bundle (lamad → shell `/epr`): plain `href`/`EprNavService` per app CLAUDE.md — NEVER
`routerLink` to another bundle. Use the EPR-native link mechanism (`eprToUniversalHref` / claims
minting) with the `/raw` subview.
**TDD:** if a URL-builder fn exists, unit-test it emits `/epr/{id}/raw`; else assert the template
anchor `href` in a component spec.
**Verify:** `pnpm --filter lamad test` (or the lamad project's vitest); lint.

### Task 6 — a2o: regression scenario (the contradiction guard)
**Files:** `genesis/a2o/features/lms/deep-link-delivery.feature` (+ steps).
**Approach:** scenario(s): (a) `/epr/{path-id}/raw` renders the raw-node view (`data-testid=epr-raw-node`),
NOT a 302 to the pillar mount; (b) from the path overview, clicking "View as Content" lands on
`/epr/{id}/raw` (the round-trip is gone). Tag `@regression` (guards the captured contradiction).
**Verify:** the scenario is authored + step-wired (Opus authors the Feature/Scenario narrative;
Sonnet/Haiku wire steps). Live E2E runs in CI; locally typecheck the steps.

## Done when

- `/epr/{id}/raw` serves the shell (never 302) and the shell renders the raw-node inspector
  (`data-testid=epr-raw-node`) with CID + contentFormat + related-id links + provenance.
- "View as Content" on a claimed-type overview lands on `/epr/{id}/raw`, not back at the pillar mount.
- All gates green (doorway: cargo test/clippy/fmt; app+lamad: vitest+lint; storage: contract if touched).
- `@regression` a2o scenario authored. The `epr-routing-complementary-captures.md:41` contradiction
  is closed by behavior, not just by design.

## Out of scope (Slices 1–4)

Format-driven focal dispatch + the three-leg lens-complete `/epr/{cid}` (Slice 1); the process leg
enforced at compose (Slice 2); CID-canonical/slug-alias + version pinning (Slice 3); offline-floor
hardening (Slice 4). Full typed-relation-neighborhood closure walk is Slice 1, not here.
