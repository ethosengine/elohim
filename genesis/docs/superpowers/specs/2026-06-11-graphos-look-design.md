# graphos-look: render the design guide & component library for agent eyes

**Date:** 2026-06-11
**Status:** Approved (operator-reviewed design, this doc is the written record)
**Owner surface:** `genesis/a2o/scripts/graphos.ts` (new), `pnpm graphos`

## Motivation

Frontend review/refinement work is eyes-first: render the surface before reading
its source. Three look surfaces exist — the app(s), Storybook (the emergent
component library), and the graphos design guide. The `look` primitive
(`scripts/look.ts`) covers any URL, and the deployed Storybook at
`https://storybook.elohim.host` exposes all 483 graphos stories
(`default-*` Library A blank-slate, `designed-*` Library B themed,
narrative/foundations guide pages) renderable standalone via
`iframe.html?id=<story-id>`. What's missing is ergonomics: enumerating what
exists, rendering a story by id without hand-building iframe URLs, and
absorbing a component's full cell/theme matrix in ONE image Read instead of
twelve. This tool makes graphos's visual cues a first-class input to the
frontend workflow.

## Decisions (operator-confirmed)

1. **Base-URL-agnostic.** Defaults to the deployed storybook (zero setup,
   shows graphos as merged to dev); `--base http://localhost:6006` targets a
   locally running `pnpm storybook` for in-branch iteration. No auto-start —
   the one manual step is documented.
2. **Three verbs: `list` / `story` / `sheet`.** Enumerate, render one
   precisely, or survey a component's whole matrix as a composite.
3. **Approach B:** sibling primitive in a2o reusing the exported `runLook()`.
   `look.ts` stays a pure URL-renderer; all storybook semantics live in
   `graphos.ts`. Zero new dependencies.

## CLI surface

```
pnpm graphos list [filter] [--base <url>]
pnpm graphos story <story-id> [--docs] [--base <url>] [--viewport WxH] [--out <slug>]
pnpm graphos sheet <component> [--family designed|default] [--cell WxH] [--cols N]
                   [--base <url>] [--out <slug>]
```

- `list` — fetch `<base>/index.json`, filter ids by substring, print grouped
  by component with counts. Text output, no browser.
- `story` — validate the id against `index.json`, resolve
  `<base>/iframe.html?id=<id>&viewMode=<mode>`, delegate to `runLook()`.
  `<mode>` is derived from the index entry's `type` (`story`/`docs`), so MDX
  guide pages render correctly by default; `--docs` forces `viewMode=docs`.
  Default out slug = story id.
- `sheet` — composite matrix image; see mechanics below. Default out slug =
  `sheet-<component>`.

## Story-id conventions

Ids follow `<family>-<group>-<component>--<cell>` (e.g.
`designed-core-elohim-compute-tile--standard`). Component matching is
segment-aligned, not substring — with a single-segment guard (ratified during
implementation, the test is canonical): a multi-segment name matches when the
prefix (before `--`) equals it or ends with `-<name>`; a single-segment name
(no dash — custom-element names always contain one) requires exact prefix
equality, so `tile` does NOT match `elohim-compute-tile`. When both
`default-*` and `designed-*` families match, the
sheet renders BOTH grouped into labeled sections — Library A vs Library B
side by side is itself a review cue. `--family` narrows. Docs-type entries
are excluded from sheets (full pages; use `story --docs`).

## Sheet mechanics

`graphos.ts` writes a self-contained `sheet.html` into the output directory:
a CSS-grid page where each cell is a labeled
`<iframe src="<base>/iframe.html?id=<story-id>&viewMode=story">` with a
caption bar (cell/variant name); family sections get header rows. It then
calls `runLook()` on the file's `file://` URL — one navigation, one full-page
screenshot → one composite `shot.png`. A `file://` parent loading http(s)
iframes is permitted in Chromium (mixed-content rules block the reverse).

- Geometry: fixed cells, default `420x320`, `--cell WxH` overrides; `--cols N`
  (default 3). Page width = cols × cell width; full-page shot captures all rows.
- Viewport covers the FULL sheet height (smoke-discovered: Chromium defers
  rendering of offscreen iframes — with a short viewport, networkidle fires
  early and below-the-fold cells capture blank with zero errors). Height is
  computed from the grid geometry, capped at 15,000px with a warning when a
  sheet exceeds it (narrow with `--family` or raise `--cols`).
- Wait: `runLook`'s `waitUntil: 'networkidle'` spans child frames; `runLook`
  gained an additive `timeoutMs` option (`--timeout` on look's CLI) and sheets
  pass a size-scaled budget (60s + 3s/iframe, cap 300s). When the budget
  expires with ZERO captured errors on a full-height viewport, the sheet exits
  0 — a persistent storybook connection can keep networkidle from ever firing
  on an otherwise-complete page (precondition documented in code: this is only
  safe BECAUSE the viewport covers every iframe).
- Capture: `page.on('response')` sees subframe responses, so a story that
  404s/throws inside its iframe lands in `capture.json` (`httpErrors`,
  console) instead of silently rendering blank.
- `sheet.html` is a persisted artifact — the operator opens the same grid
  (live iframes) via `pnpm reports:serve`.

## Data flow

`index.json` → filter/group (pure functions) → `sheet.html` or iframe URL →
`runLook()` → `reports/look/<slug>/{shot.png, capture.json[, sheet.html]}`.

## Error handling

- `index.json` unreachable → message distinguishes deployed (site/network
  down) vs local ("no storybook at localhost:6006 — start it with:
  `cd app/elohim-library && pnpm storybook`"). Non-zero exit.
- Unknown component/filter/story-id → nearest-match suggestions (substring
  scan over component prefixes); story ids validated BEFORE browser launch.
- Missing Playwright browser → `runLook` already throws `pnpm a2o:setup`
  guidance; inherited.
- A story erroring inside its iframe is NOT a sheet failure — sheet renders,
  evidence lands in `capture.json` ("capture what rendered" philosophy).

## Testing

Pure functions (id parsing, component matching + suggestions, grouping,
`sheet.html` generation) unit-tested in `scripts/__tests__/graphos-stories.test.ts`
+ `scripts/__tests__/graphos-cli.test.ts` (parser, slug containment, groupRows)
under a2o's existing `test:unit` (node:test via tsx). Browser paths follow
the `look.ts` convention: exercised by use, not CI. Post-implementation
smoke: `pnpm graphos sheet elohim-compute-tile` rendered and Read.

## Touchpoints

- `genesis/a2o/package.json` — one script line: `"graphos": "tsx scripts/graphos.ts"`.
- `genesis/a2o/CLAUDE.md` — Tools bullet beside the `look` entry.
- Root `CLAUDE.md` — "Frontend Eyes (available rails)" subsection: pointers so
  any agent discovers what is AVAILABLE (look, graphos, storybook,
  reports:serve) when planning frontend design changes — capability
  discovery, not prescription.
- Memory `frontend-review-eyes-first` — names `pnpm graphos` for the
  Storybook/design-guide surfaces.

## P2P design gate

Not triggered: no data entities (no tables, models, routes, DHT entries, or
sync messages). This is agent tooling — a CLI script producing local report
artifacts.

## Out of scope (YAGNI)

Local storybook auto-start; before/after visual diffing; image-stitching
dependencies; authenticated stories (storybook is public); any
Jenkinsfile/orchestrator wiring.
