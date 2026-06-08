# A11y contrast gate for the Angular shell — close the dark-on-dark blind spot

**Status:** Layer 0+1 landed (2026-06-08) · Layer 2 + legacy migration captured · **Priority:** medium-high (every new shell component can reintroduce the bug)

## The incident that motivated this

`/epr/{id}/raw` (EprRawNodeComponent, EPR Slice 0) rendered **near-black text on the
dark constellation canvas** — unreadable in dark theme. Root cause: the component set
`color: var(--epr-raw-fg, #1f2430)` with **no paired background**; `--epr-raw-fg` was
never defined, so it resolved to the hardcoded near-black literal in BOTH themes, and the
dark page came from `color-scheme: dark` painting the *canvas* (not a `background-color`).
Fixed by binding to the reactive `--lamad-*` tokens + a self-contained panel surface
(measured 0 WCAG-AA failures in both themes). Same-cause siblings fixed in the same pass:
`epr-relationship-card`, `epr-relationships-panel`, `exploration-sidebar`,
`related-concepts-panel` (the last two were using a generic `--text-*/--surface-*` Material
namespace, never reactive in lamad).

## Why no gate caught it

- **axe-core is structurally blind to it.** Run against the live broken page → **0
  violations**. axe resolves an element's background by walking ancestors for a
  `background-color`; html/body were `transparent` and the dark came from the
  `color-scheme` canvas, which axe can't read → it assumes white and scores near-black on
  white = pass. The failure was dark-on-dark to a human, light-on-white to axe.
- **The real contrast gate is elohim-core-only.** The computed-style WCAG walk
  (`elohim-core` theme-contrast helper, colorjs.io, dark+light, axe-strict) guards 13 Lit
  elements. The Angular **shell** has no equivalent.
- **stylelint-a11y was inert.** Rules were configured with ESLint-style `'warn'`/`'error'`
  strings (invalid stylelint config) → `Invalid Option` → silently disabled. *(Fixed in
  Layer 0.)*
- **SonarQube S7924 is static** — can't resolve `var()` chains across themes or model the
  canvas.
- **husky pre-push ran no contrast/a11y check at all.**

## Layered remedy

| Layer | What | Status |
|---|---|---|
| **0 — repair** | Fix the inert `stylelint-a11y` config (`true`/`{severity}` not `'warn'`) so focus/motion/font-size rules actually run | **DONE 2026-06-08** (`app/elohim-app/.stylelintrc.js`) |
| **1 — static canary** | `scripts/lint-a11y-color.mjs`: a color/background literal must chain to a theme token (`--lamad-*/--elohim-*/--el-*/--primary/--accent/--text-light`), else fail. `a11y-color-ok:` pragma for sanctioned keepers (semantic status, scrims). Wired into `app/elohim-app/justfile` `gate` in **ratchet** mode (`--diff` vs dev base): grandfathers the legacy debt, blocks new literals. `lint-a11y-strict` runs full over already-clean subtrees. | **DONE 2026-06-08** |
| **2 — dynamic gate** | Productionize the both-themes render contrast gate for shell routes: render each route at `data-theme=dark` and `light` over the **real canvas background**, run the computed-style WCAG walk (the shell twin of the elohim-core theme-contrast gate). This is the ONLY thing that catches the `color-scheme`/canvas class axe misses. Needs a Playwright lane in elohim-app CI. A working prototype harness was used to verify the raw-node fix this session. | **TODO** |

## The legacy debt the ratchet grandfathers

`lint-a11y-color.mjs` (full, non-ratchet) over `app/elohim-app/src` reports **~1086 hardcoded
color literals across 96 component files** not bound to a theme token — concentrated in
`imagodei/components` (387), `qahal/components` (243), `elohim/components` (236),
`avodah/components` (81), `shefa/components` (66). Most are theme-blind (light-only) or
semantic status colors. This is a **migration, not a gate** — the ratchet holds the line
while it's worked down. Migration approach: per-pillar, rebind to `--lamad-*` (matching the
theme-correct `content-viewer`), pragma genuine semantic/scrim keepers, run
`lint-a11y-strict <pillar-dir>` to confirm a subtree is clean, then promote that subtree to
the strict (non-ratchet) gate.

## Pointers
- Canary: `scripts/lint-a11y-color.mjs` · route-literal sibling: `scripts/lint-route-literals.mjs`
- elohim-core contrast gate (the Layer-2 template): `app/elohim-elements/elohim-core` theme-contrast helper
- Theme-authority contract (the gospel rule "every bound *-bg paired with a bound *-fg"; `html[data-theme]` authority): `app/elohim-app/CLAUDE.md` §Chrome & cross-bundle composition rails
