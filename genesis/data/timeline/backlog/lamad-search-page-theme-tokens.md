---
id: "backlog-lamad-search-page-theme-tokens"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Search page chrome never joined the token layer — dark-scheme literals read fine, light scheme goes near-invisible"
slug: "lamad-search-page-theme-tokens"
written: "2026-09-24"
author: "agent:implementer@claude-sonnet-5 (Lane S)"
status: "backlog"
priority: "low"
domain: "frontend (graphos/theming)"
area: "app/lamad search component styles"
tags: [search, theming, graphos, lamad, grandfathered-literals, look-evidence]
relatedNodeIds:
  - content-search-station-4
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - genesis/a2o/reports/look/s8-search-offline/shot.png
  - app/lamad/src/app/components/search/search.component.ts
shift_objective: |
  Bind the search page's component styles onto the graphos token layer so the light and dark
  color schemes both render legibly. Start from the one line already token-bound
  (`var(--lamad-text-secondary, #64748b)`) and extend the same pattern to the remaining
  grandfathered hex literals in `search.component.ts` (`#e0e6ed`, `#6366f1`, `#8b5cf6`,
  `#94a3b8`, `#a5b4fc`). Verify with `pnpm look` against both color schemes, not source-reading
  alone — the defect is visual-only and was invisible in a code review of the dark-scheme render.
---

# The search page's chrome never joined the token layer

**Chain:** search page renders → the household's chosen chrome theme.

**Between (A→C):** the search route (`app/lamad/src/app/components/search/search.component.ts`)
renders its own component styles → the graphos theme-token layer the rest of the app's chrome
draws from.

**Missing node:** the search component was authored straight to hardcoded hex literals and never
joined the token layer, so it renders correctly by accident on the one scheme its author's editor
theme happened to resemble (dark) and fails on the other.

**Probe:** the light-scheme `pnpm look` capture at
`genesis/a2o/reports/look/s8-search-offline/shot.png` shows dark result cards and a near-invisible
heading sitting on the light chrome, while the dark-scheme render of the same page is clean —
eyes-first review caught what a source read of the component alone would not have (the code
looks plausible; only the light-scheme render exposes the mismatch). A grep-verified count of
grandfathered hex-literal colors remains in `search.component.ts`:

```
2  #6366f1
2  #64748b   (one still bare on line ~170, the other already the fallback of a bound var)
1  #8b5cf6
2  #94a3b8
1  #a5b4fc
3  #e0e6ed
```

Eleven literal occurrences across six distinct colors (re-counted at authoring time — treat this
number, not any earlier verbal estimate, as current). Exactly one has joined the token layer:
`color: var(--lamad-text-secondary, #64748b)` at line 120, where the hex is now a documented
fallback rather than the live value. The rest are un-owned hardcodes with no token behind them.

**Current state:** the search page is new (landed in `c8f8fcad8`, the content-search sprint) and
was built and reviewed against a single rendered scheme. No open item names the remaining ten
literal occurrences as a theming gap; this atom is that name, so the next pass through the
component has a concrete checklist rather than re-discovering the same near-invisible heading by
eye a second time.
