---
view: /doorway
urls: [https://doorway-alpha.elohim.host/doorway]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/doorway/ (pillar alias @app/doorway)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/doorway --as Adam --out recheck-doorway-operator-console --budget 180
---
# Doorway operator console: persona walk

A member who opens `/doorway` gets the operator dashboard with no bar and no way out.

## Stations

- [ ] POLISH·H8 Label the operator console as an operator view and stop a member reading zeros as failure
  - Persona / route: Adam, `/doorway`
  - Repro: 1. Load `/doorway` as Adam. 2. Read the heading, tiles and left menu.
  - Expected (as persona): either nothing here for a member, or a clearly labelled operator view with a way back.
  - Observed: "Doorway Operator Dashboard" with Nodes 1/1, trust 0.0%, impact 0.0%, success 0.0%, an empty Overview tab, and a left menu "Dashboard / Configuration" with no way out; no top bar. Verify first: the zeros may come from the `/hc/connect` 401s (RAM-guard also shed processes during this render).
  - Source: `app/elohim-app/src/app/doorway/`; `grep -rn "Doorway Operator Dashboard" /projects/elohim/app/elohim-app/src`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: decision recorded in `genesis/a2o/walks/doorway-operator-console.md` stating whether members see this route (hide, gate by role, or label "Operator view"); the page carries the shared header with a way home, and unavailable metrics render as "not available" rather than 0.0%.
  - Finding ids: SHELL-14
