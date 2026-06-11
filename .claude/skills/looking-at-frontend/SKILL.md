---
name: looking-at-frontend
description: Use when reviewing, designing, debugging, or modifying any UI surface — Angular app pages, Lit elohim-elements, graphos Storybook stories (Library A default / Library B designed), the design guide — when judging visual quality, aesthetics, theming, or brand drift, or when a described feature can't be found in the rendered app. Especially when you're about to give a visual verdict from source files alone.
---

# Looking at Frontend

## Overview

**Eyes first.** Render the actual surface before reading source or giving any visual verdict. Token values can be byte-correct in source while the binding is dead at runtime — wrong property name, specificity, missing webfont. Real case: a designed story bound `--elohim-reaction-count-color`, but the primitive's surface is `--elohim-reaction-count-fg` — byte-perfect tokens, dead binding, visible only in the light-theme render (dark mode accidentally masked it via `color-scheme` fallbacks).

All render tooling lives in `genesis/a2o`. Artifacts land in `genesis/a2o/reports/look/<slug>/` — `shot.png` (Read it directly) + `capture.json` (console, pageErrors, httpErrors, network). `pnpm reports:serve` exposes them to the operator.

## The three look surfaces

| Surface | Command (from `genesis/a2o`) | Notes |
|---|---|---|
| **The app** | `pnpm look <url> [--as <FixtureHuman>] [--viewport WxH] [--out slug]` | Renders ANY url, auth-aware. Targets: local stack (`pnpm hc:start` + app :4200), `pnpm start:alpha` in `app/elohim-app` (local UI × live alpha data, read-mostly), or deployed alpha. |
| **Pattern libraries (Storybook)** | `pnpm graphos list [filter]` (enumerate, no browser) · `pnpm graphos story <story-id>` (one story) · `pnpm graphos sheet <component>` (full cell/theme matrix, both libraries, one composite image — fastest design-language absorption) | Story ids: `default-*` = Library A blank-slate, `designed-*` = Library B themed, `foundations-*`, narrative `i-iv`. |
| **Design guide** | `pnpm graphos story <docs-id>` (MDX pages; viewMode auto-derived, `--docs` forces) | Absorb the established aesthetic BEFORE designing or refining. |

**Base matters:** `graphos` defaults to the deployed storybook (storybook.elohim.host) = graphos **as merged to dev**. For in-branch work, run `pnpm storybook` in `app/elohim-library` and pass `--base http://localhost:6006`. Judging branch changes against the deployed base is a category error.

## Reading a render

- Render **every claimed state**: both themes minimum, plus lens/cell variants — `sheet` does the matrix in one shot.
- Cross-check designed-story bindings against the primitive's `@cssprop` surface in `app/elohim-elements/` — stories can bind names the primitive never declares.
- Check `capture.json` for pageErrors/httpErrors before trusting what you see.

## Can't-find ≠ never-implemented

If the view the operator describes doesn't show up, suspect present-moment reachability — broken route, failed data load, 403/404 in `capture.json` httpErrors, env — and investigate before concluding the feature doesn't exist.

## Known blind spots of a static render

- No hover/focus/motion — judge motion via code + spec, or interactive runs (`pnpm test:browser`).
- The headless container has no color-emoji font — emoji render as boxes; don't grade iconography from that.

## Common mistakes

| Mistake | Reality |
|---|---|
| "`pnpm look` only targets the app, so I can't render Storybook" | `look` renders any URL; `graphos` wraps the storybook surfaces specifically. |
| Visual verdict from source tokens alone | The dead-binding class is invisible on paper. Render, then read. |
| Rendering only one theme | Dark `color-scheme` fallbacks can mask light-theme bugs (and vice versa). |
| "Feature isn't implemented" after one failed look | Reachability first: httpErrors, routes, git history. |

## Dispatching frontend work to subagents

Subagents do NOT self-discover skills (observed twice: agents reviewed designed stories code-only, claiming "`pnpm look` doesn't apply to Storybook" / "no browser tool" — both false). When dispatching any visual review/design/verification task, the dispatch prompt MUST carry the eyes rail explicitly, e.g.: *"Render before judging: from `genesis/a2o`, `pnpm graphos story <id>` / `sheet <component>` → Read the shot.png. Full rails: `.claude/skills/looking-at-frontend/SKILL.md`."* The `component-architect` and `graphos-designer` agent bodies carry this pointer; generic agents need it in the prompt.

## Integration

- a11y/testid legibility audit → `page-model` skill.
- Screenshot-vs-promise delivery judgment → `/deliver`.
- Greenfield aesthetics → `frontend-design` plugin skill; **inside graphos the aesthetic is already chosen** (`genesis/graphos/elohim-protocol-design-spec.md`).
- Library A/B ownership + story conventions → `app/elohim-library/CLAUDE.md`. Tooling spec: `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`.
