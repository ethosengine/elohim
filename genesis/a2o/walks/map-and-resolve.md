---
view: /map, /resolve
urls: [https://doorway-alpha.elohim.host/map, https://doorway-alpha.elohim.host/resolve, https://doorway-alpha.elohim.host/epr/]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/**/map component, app/elohim-app/src/app/**/resolve (route /resolve redirecting to /epr/)
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-shell
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/resolve --as Adam --out recheck-map-and-resolve --budget 180
---
# Map and resolve: persona walk

Two utility routes with no top bar. `/map` needs WebGL2, which headless Chromium lacks, so its message is partly environmental. `/resolve` with no address shows a broken sentence.

## Stations

- [ ] CONFUSES·H9 Fix the empty interpolation and give bare `/resolve` an input
  - Persona / route: Adam, `/resolve` (redirects to `/epr/`)
  - Repro: 1. Load `/resolve` as Adam. 2. Read the page.
  - Expected (as persona): a lookup box, or a clear instruction.
  - Observed: a redirect to `/epr/` showing "We can't reach this one from here / No peer ... is holding ." The slot where the name should be is empty, with a stray space before the period; no way back and no input. Note the RAM guard shed processes during the shell walk, so re-capture once to confirm.
  - Source: `grep -rn "is holding" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: `/resolve` with no address shows an input (or a link to where to find an address) and never renders "No peer … is holding ."; with an empty name the unreachable message is not shown at all; recheck page text confirms.
  - Finding ids: SHELL-12

- [ ] POLISH·H9 Give `/map` a non-WebGL fallback and label its two floating buttons
  - Persona / route: anonymous and Adam, `/map`
  - Repro: 1. Load `/map` in a browser without WebGL2. 2. Read the page and the two round buttons.
  - Expected (as persona): a list of places, or at least a clear message with a way on.
  - Observed: "Map unavailable — this browser or environment does not support WebGL." readable on an empty gradient, with two unlabelled round floating buttons (layers, locate) and a WebGL2 warning stack in the console. Verify first: the missing WebGL2 is environmental (headless); only the missing fallback and the unlabelled buttons are real. The page also has no top bar (see walks__product-tokens-and-shell#1).
  - Source: `grep -rn "Map unavailable" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: with WebGL disabled the page lists places (or links to a list) beneath the message, and both floating buttons have `aria-label`; recheck DOM confirms.
  - Finding ids: SHELL-13, ANON-21
